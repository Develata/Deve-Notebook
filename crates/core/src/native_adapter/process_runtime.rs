//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-process-adapter-decision
//!   - 08_ui_design_03_mobile#mobile-process-adapter-decision

use std::fmt;
use std::path::{Component, Path, PathBuf};

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;

use super::{NativeEndpointReady, NativeProcessAdapterPolicy, NativeServiceHealthProbe};

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct NativeProcessEnvBinding {
    pub key: String,
    pub value: String,
}

impl fmt::Debug for NativeProcessEnvBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeProcessEnvBinding")
            .field("key", &self.key)
            .field("value", &redacted_env_value(&self.key, &self.value))
            .finish()
    }
}

impl Serialize for NativeProcessEnvBinding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("NativeProcessEnvBinding", 2)?;
        state.serialize_field("key", &self.key)?;
        state.serialize_field("value", redacted_env_value(&self.key, &self.value))?;
        state.end()
    }
}

fn redacted_env_value<'a>(key: &str, value: &'a str) -> &'a str {
    let key = key.to_ascii_uppercase();
    if key.contains("SECRET") || key.contains("TOKEN") || key.contains("PASS") {
        "<redacted>"
    } else {
        value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeProcessPathResolution {
    AbsoluteOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeProcessBindHints {
    pub http_host: String,
    pub http_port: Option<u16>,
    pub ws_host: String,
    pub ws_port: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeProcessSpawnSpec {
    pub executable: PathBuf,
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub env_allowlist: Vec<String>,
    pub env: Vec<NativeProcessEnvBinding>,
    pub profile: String,
    pub config_path: PathBuf,
    pub ledger_path: PathBuf,
    pub bind_hints: NativeProcessBindHints,
    pub path_resolution: NativeProcessPathResolution,
}

impl NativeProcessSpawnSpec {
    pub fn validate_contract(&self) -> Result<(), NativeProcessRuntimeError> {
        match self.path_resolution {
            NativeProcessPathResolution::AbsoluteOnly => {
                validate_absolute_path("executable", &self.executable)?;
                validate_absolute_path("cwd", &self.cwd)?;
            }
        }
        validate_absolute_path("config_path", &self.config_path)?;
        validate_absolute_path("ledger_path", &self.ledger_path)?;
        validate_bind_hints(&self.bind_hints)?;
        validate_env_contract(&self.env_allowlist, &self.env)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeProcessRuntimeHandle {
    pub handle_id: String,
    pub platform_pid: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeProcessRuntimeState {
    Disabled,
    SpawnRequested,
    Spawned,
    EndpointProbing,
    EndpointHealthy,
    SessionHandoffReady,
    RuntimeReady,
    Restarting,
    Offline,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeProcessRuntimeFailureKind {
    SpawnExecutableMissing,
    SpawnPermissionDenied,
    InvalidExecutablePath,
    InvalidWorkingDirectory,
    EnvironmentPolicyViolation,
    BindFailed,
    HealthProbeFailed,
    ProcessExited,
    SessionHandoffFailed,
    NonLoopbackEndpoint,
}

impl NativeProcessRuntimeFailureKind {
    pub fn retryable_by_default(self) -> bool {
        matches!(
            self,
            Self::BindFailed | Self::HealthProbeFailed | Self::ProcessExited
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeProcessExitStatus {
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeProcessRuntimeEvent {
    pub state: NativeProcessRuntimeState,
    pub timestamp_unix_ms: i64,
    pub failure: Option<NativeProcessRuntimeFailureKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeProcessRuntimeSnapshot {
    pub state: NativeProcessRuntimeState,
    pub handle: Option<NativeProcessRuntimeHandle>,
    pub endpoint: Option<NativeEndpointReady>,
    pub health_probe: NativeServiceHealthProbe,
    pub started_at_unix_ms: Option<i64>,
    pub exit_status: Option<NativeProcessExitStatus>,
    pub last_failure: Option<NativeProcessRuntimeFailureKind>,
    pub child_process_runtime_enabled: bool,
    pub authority_writes_allowed: bool,
}

impl NativeProcessRuntimeSnapshot {
    pub fn disabled_by_policy(policy: NativeProcessAdapterPolicy) -> Self {
        Self {
            state: NativeProcessRuntimeState::Disabled,
            handle: None,
            endpoint: None,
            health_probe: NativeServiceHealthProbe::default(),
            started_at_unix_ms: None,
            exit_status: None,
            last_failure: None,
            child_process_runtime_enabled: policy.child_process_runtime_enabled,
            authority_writes_allowed: policy.authority_writes_allowed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NativeProcessRuntimeError {
    #[error("native process runtime is disabled by process adapter policy")]
    RuntimeDisabled,
    #[error("{field} must not be empty")]
    EmptyPath { field: &'static str },
    #[error("{field} must be absolute in native process spawn spec")]
    RelativePathForbidden { field: &'static str },
    #[error("{field} must not contain parent directory components")]
    ParentDirForbidden { field: &'static str },
    #[error("native process environment key must not be empty")]
    EmptyEnvironmentKey,
    #[error("native process environment key must not contain '=': {key}")]
    InvalidEnvironmentKey { key: String },
    #[error("native process environment variable is not allowlisted: {key}")]
    EnvironmentVariableNotAllowlisted { key: String },
    #[error("{field} host must be 127.0.0.1 or localhost")]
    NonLoopbackBindHost { field: &'static str },
    #[error("{field} port must be non-zero")]
    InvalidBindPort { field: &'static str },
}

fn validate_absolute_path(
    field: &'static str,
    path: &Path,
) -> Result<(), NativeProcessRuntimeError> {
    if path.as_os_str().is_empty() {
        return Err(NativeProcessRuntimeError::EmptyPath { field });
    }
    if !path.is_absolute() {
        return Err(NativeProcessRuntimeError::RelativePathForbidden { field });
    }
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(NativeProcessRuntimeError::ParentDirForbidden { field });
    }
    Ok(())
}

fn validate_bind_hints(hints: &NativeProcessBindHints) -> Result<(), NativeProcessRuntimeError> {
    validate_bind_host("http_host", &hints.http_host)?;
    validate_bind_host("ws_host", &hints.ws_host)?;
    validate_bind_port("http_port", hints.http_port)?;
    validate_bind_port("ws_port", hints.ws_port)
}

fn validate_bind_host(field: &'static str, host: &str) -> Result<(), NativeProcessRuntimeError> {
    if matches!(host, "127.0.0.1" | "localhost") {
        Ok(())
    } else {
        Err(NativeProcessRuntimeError::NonLoopbackBindHost { field })
    }
}

fn validate_bind_port(
    field: &'static str,
    port: Option<u16>,
) -> Result<(), NativeProcessRuntimeError> {
    if port.is_some_and(|port| port == 0) {
        Err(NativeProcessRuntimeError::InvalidBindPort { field })
    } else {
        Ok(())
    }
}

fn validate_env_contract(
    allowlist: &[String],
    env: &[NativeProcessEnvBinding],
) -> Result<(), NativeProcessRuntimeError> {
    for key in allowlist {
        validate_env_key(key)?;
    }
    for binding in env {
        validate_env_key(&binding.key)?;
        if !allowlist.iter().any(|allowed| allowed == &binding.key) {
            return Err(
                NativeProcessRuntimeError::EnvironmentVariableNotAllowlisted {
                    key: binding.key.clone(),
                },
            );
        }
    }
    Ok(())
}

fn validate_env_key(key: &str) -> Result<(), NativeProcessRuntimeError> {
    if key.trim().is_empty() {
        return Err(NativeProcessRuntimeError::EmptyEnvironmentKey);
    }
    if key.contains('=') {
        return Err(NativeProcessRuntimeError::InvalidEnvironmentKey {
            key: key.to_string(),
        });
    }
    Ok(())
}
