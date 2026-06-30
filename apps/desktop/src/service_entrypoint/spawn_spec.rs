//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-process-adapter-decision
//!

use std::path::{Path, PathBuf};

use deve_core::config::AppProfile;
use deve_core::native_adapter::{
    NATIVE_SESSION_BOOTSTRAP_SECRET_ENV, NativeProcessBindHints, NativeProcessEnvBinding,
    NativeProcessPathResolution, NativeProcessSpawnSpec, native_tauri_allowed_origins,
};
use deve_core::security::auth::password;

use super::{DesktopLocalServiceEntrypointError, DesktopLocalServiceEntrypointInput};

const DEVE_PLUGIN_DIR_ENV: &str = "DEVE_PLUGIN_DIR";

pub(super) fn build_spawn_spec(
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
            value: native_tauri_allowed_origins().join(","),
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
