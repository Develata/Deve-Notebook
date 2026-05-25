//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-process-adapter-decision

use std::net::TcpListener;
use std::path::{Path, PathBuf};

use deve_core::config::AppProfile;
use deve_core::native_adapter::{
    NATIVE_SESSION_BOOTSTRAP_SECRET_ENV, NativeProcessAdapterDecision, NativeProcessAdapterPolicy,
    NativeProcessBindHints, NativeProcessEnvBinding, NativeProcessPathResolution,
    NativeProcessRuntimeError, NativeProcessSpawnSpec,
};
use deve_core::security::auth::password;
use thiserror::Error;

pub const DEVE_DESKTOP_LOCAL_SERVICE_ENV: &str = "DEVE_DESKTOP_LOCAL_SERVICE";
const DESKTOP_SERVICE_MAX_RESTART_ATTEMPTS: u32 = 2;
const DESKTOP_TAURI_ORIGIN: &str = "http://tauri.localhost";
const DEVE_PLUGIN_DIR_ENV: &str = "DEVE_PLUGIN_DIR";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopLocalServiceEntrypointPolicy {
    pub opt_in: bool,
    pub child_process_runtime_enabled: bool,
    pub max_restart_attempts: u32,
}

impl DesktopLocalServiceEntrypointPolicy {
    pub fn disabled() -> Self {
        Self {
            opt_in: false,
            child_process_runtime_enabled: false,
            max_restart_attempts: DESKTOP_SERVICE_MAX_RESTART_ATTEMPTS,
        }
    }

    pub fn opt_in_enabled() -> Self {
        Self {
            opt_in: true,
            child_process_runtime_enabled: true,
            max_restart_attempts: DESKTOP_SERVICE_MAX_RESTART_ATTEMPTS,
        }
    }

    pub fn native_policy(self) -> NativeProcessAdapterPolicy {
        NativeProcessAdapterPolicy {
            decision: NativeProcessAdapterDecision::DeferredUntilPackagingGate,
            child_process_runtime_enabled: self.child_process_runtime_enabled,
            packaging_gate_required: true,
            authority_writes_allowed: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopLocalServiceEntrypointInput {
    pub current_exe: PathBuf,
    pub data_root: PathBuf,
    pub port: u16,
    pub profile: AppProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopLocalServiceEntrypointPlan {
    pub policy: DesktopLocalServiceEntrypointPolicy,
    pub spawn_spec: NativeProcessSpawnSpec,
    pub http_base: String,
    pub ws_base: String,
    pub health_probe_required_before_bootstrap: bool,
    pub session_handoff_required_before_bootstrap: bool,
    pub opens_authority_write_path: bool,
}

#[derive(Debug, Error)]
pub enum DesktopLocalServiceEntrypointError {
    #[error("desktop local service opt-in value is invalid: {value}")]
    InvalidOptInValue { value: String },
    #[error("desktop executable path has no parent directory")]
    MissingExecutableParent,
    #[error("failed to resolve desktop process path")]
    ProcessPathFailed(#[source] std::io::Error),
    #[error("failed to allocate a loopback port")]
    PortAllocationFailed(#[source] std::io::Error),
    #[error("failed to generate native session bootstrap secret")]
    SessionSecretGenerationFailed,
    #[error("failed to generate native auth material")]
    SessionAuthMaterialGenerationFailed,
    #[error(transparent)]
    InvalidSpawnSpec(#[from] NativeProcessRuntimeError),
}

pub fn desktop_local_service_entrypoint_policy_from_env()
-> Result<DesktopLocalServiceEntrypointPolicy, DesktopLocalServiceEntrypointError> {
    let Some(value) = std::env::var_os(DEVE_DESKTOP_LOCAL_SERVICE_ENV) else {
        return Ok(DesktopLocalServiceEntrypointPolicy::disabled());
    };
    parse_opt_in_value(&value.to_string_lossy())
}

pub fn plan_desktop_local_service_entrypoint(
    policy: DesktopLocalServiceEntrypointPolicy,
    input: DesktopLocalServiceEntrypointInput,
) -> Result<Option<DesktopLocalServiceEntrypointPlan>, DesktopLocalServiceEntrypointError> {
    if !policy.opt_in {
        return Ok(None);
    }

    let spawn_spec = build_spawn_spec(&input)?;
    spawn_spec.validate_contract()?;
    Ok(Some(DesktopLocalServiceEntrypointPlan {
        http_base: format!("http://127.0.0.1:{}", input.port),
        ws_base: format!("ws://127.0.0.1:{}", input.port),
        spawn_spec,
        policy,
        health_probe_required_before_bootstrap: true,
        session_handoff_required_before_bootstrap: true,
        opens_authority_write_path: false,
    }))
}

pub fn plan_desktop_local_service_entrypoint_from_env()
-> Result<Option<DesktopLocalServiceEntrypointPlan>, DesktopLocalServiceEntrypointError> {
    let policy = desktop_local_service_entrypoint_policy_from_env()?;
    if !policy.opt_in {
        return Ok(None);
    }

    let current_exe =
        std::env::current_exe().map_err(DesktopLocalServiceEntrypointError::ProcessPathFailed)?;
    let data_root =
        std::env::current_dir().map_err(DesktopLocalServiceEntrypointError::ProcessPathFailed)?;
    let port = allocate_loopback_port()?;
    plan_desktop_local_service_entrypoint(
        policy,
        DesktopLocalServiceEntrypointInput {
            current_exe,
            data_root,
            port,
            profile: AppProfile::Standard,
        },
    )
}

fn build_spawn_spec(
    input: &DesktopLocalServiceEntrypointInput,
) -> Result<NativeProcessSpawnSpec, DesktopLocalServiceEntrypointError> {
    let executable = packaged_cli_sibling(&input.current_exe)?;
    let plugin_dir = packaged_plugin_sibling_dir(&input.current_exe)?;
    let ledger_path = input.data_root.join("ledger");
    let native_session_secret = generate_native_session_bootstrap_secret()?;
    let native_auth_secret = generate_native_session_bootstrap_secret()?;
    let native_auth_password = generate_native_session_bootstrap_secret()?;
    let native_auth_password_hash = password::hash_password(&native_auth_password)
        .map_err(|_| DesktopLocalServiceEntrypointError::SessionAuthMaterialGenerationFailed)?;
    let platform_env = platform_required_child_env();
    let mut env_allowlist = vec![
        "DEVE_PROFILE".to_string(),
        "DEVE_LEDGER_DIR".to_string(),
        NATIVE_SESSION_BOOTSTRAP_SECRET_ENV.to_string(),
        "AUTH_SECRET".to_string(),
        "AUTH_PASS".to_string(),
        "AUTH_USER".to_string(),
        "ALLOWED_ORIGINS".to_string(),
        DEVE_PLUGIN_DIR_ENV.to_string(),
    ];
    let mut env = vec![
        NativeProcessEnvBinding {
            key: "DEVE_PROFILE".to_string(),
            value: profile_env_value(input.profile).to_string(),
        },
        NativeProcessEnvBinding {
            key: "DEVE_LEDGER_DIR".to_string(),
            value: ledger_path.to_string_lossy().to_string(),
        },
        NativeProcessEnvBinding {
            key: NATIVE_SESSION_BOOTSTRAP_SECRET_ENV.to_string(),
            value: native_session_secret,
        },
        NativeProcessEnvBinding {
            key: "AUTH_SECRET".to_string(),
            value: native_auth_secret,
        },
        NativeProcessEnvBinding {
            key: "AUTH_PASS".to_string(),
            value: native_auth_password_hash,
        },
        NativeProcessEnvBinding {
            key: "AUTH_USER".to_string(),
            value: "native".to_string(),
        },
        NativeProcessEnvBinding {
            key: "ALLOWED_ORIGINS".to_string(),
            value: DESKTOP_TAURI_ORIGIN.to_string(),
        },
        NativeProcessEnvBinding {
            key: DEVE_PLUGIN_DIR_ENV.to_string(),
            value: plugin_dir.to_string_lossy().to_string(),
        },
    ];
    for binding in platform_env {
        env_allowlist.push(binding.key.clone());
        env.push(binding);
    }
    Ok(NativeProcessSpawnSpec {
        executable,
        argv: vec![
            "serve".to_string(),
            "--native-loopback".to_string(),
            "--port".to_string(),
            input.port.to_string(),
        ],
        cwd: input.data_root.clone(),
        env_allowlist,
        env,
        profile: profile_env_value(input.profile).to_string(),
        config_path: input.data_root.join("config.toml"),
        ledger_path,
        bind_hints: NativeProcessBindHints {
            http_host: "127.0.0.1".to_string(),
            http_port: Some(input.port),
            ws_host: "127.0.0.1".to_string(),
            ws_port: Some(input.port),
        },
        path_resolution: NativeProcessPathResolution::AbsoluteOnly,
    })
}

fn platform_required_child_env() -> Vec<NativeProcessEnvBinding> {
    if !cfg!(windows) {
        return Vec::new();
    }
    ["SystemRoot", "WINDIR"]
        .into_iter()
        .filter_map(|key| {
            env_value_case_insensitive(key).map(|value| NativeProcessEnvBinding {
                key: key.to_string(),
                value,
            })
        })
        .collect()
}

fn env_value_case_insensitive(key: &str) -> Option<String> {
    std::env::vars_os()
        .find(|(candidate, _)| candidate.to_string_lossy().eq_ignore_ascii_case(key))
        .map(|(_, value)| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty())
}

fn generate_native_session_bootstrap_secret() -> Result<String, DesktopLocalServiceEntrypointError>
{
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|_| DesktopLocalServiceEntrypointError::SessionSecretGenerationFailed)?;
    Ok(hex_encode(&bytes))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn packaged_cli_sibling(current_exe: &Path) -> Result<PathBuf, DesktopLocalServiceEntrypointError> {
    let Some(parent) = current_exe.parent() else {
        return Err(DesktopLocalServiceEntrypointError::MissingExecutableParent);
    };
    Ok(parent.join(deve_cli_binary_name()))
}

fn packaged_plugin_sibling_dir(
    current_exe: &Path,
) -> Result<PathBuf, DesktopLocalServiceEntrypointError> {
    let Some(parent) = current_exe.parent() else {
        return Err(DesktopLocalServiceEntrypointError::MissingExecutableParent);
    };
    Ok(parent.join("plugins"))
}

fn deve_cli_binary_name() -> &'static str {
    if cfg!(windows) {
        "deve_cli.exe"
    } else {
        "deve_cli"
    }
}

fn profile_env_value(profile: AppProfile) -> &'static str {
    match profile {
        AppProfile::Standard => "standard",
        AppProfile::LowSpec => "low-spec",
    }
}

fn parse_opt_in_value(
    value: &str,
) -> Result<DesktopLocalServiceEntrypointPolicy, DesktopLocalServiceEntrypointError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(DesktopLocalServiceEntrypointPolicy::opt_in_enabled()),
        "0" | "false" | "no" | "off" => Ok(DesktopLocalServiceEntrypointPolicy::disabled()),
        _ => Err(DesktopLocalServiceEntrypointError::InvalidOptInValue {
            value: value.to_string(),
        }),
    }
}

fn allocate_loopback_port() -> Result<u16, DesktopLocalServiceEntrypointError> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(DesktopLocalServiceEntrypointError::PortAllocationFailed)?;
    listener
        .local_addr()
        .map(|addr| addr.port())
        .map_err(DesktopLocalServiceEntrypointError::PortAllocationFailed)
}
