//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-process-adapter-decision

use std::net::TcpListener;
use std::path::PathBuf;

use deve_core::config::AppProfile;
use deve_core::native_adapter::{
    NativeProcessAdapterDecision, NativeProcessAdapterPolicy, NativeProcessRuntimeError,
    NativeProcessSpawnSpec,
};
use thiserror::Error;

mod spawn_spec;

use spawn_spec::build_spawn_spec;

pub const DEVE_DESKTOP_LOCAL_SERVICE_ENV: &str = "DEVE_DESKTOP_LOCAL_SERVICE";
const DESKTOP_SERVICE_MAX_RESTART_ATTEMPTS: u32 = 2;

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
