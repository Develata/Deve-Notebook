//! plan_ref:
//!   - 11_ui_design/index#native-post-gate-common-contract
//!   - 15_settings#native-host-local-backend-preference

use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use deve_core::native_adapter::{
    NativeBackendMode, NativeBackendPreference, NativeBackendValidationResult, NativeRemoteTarget,
    native_shell_mode_for_backend_preference, validate_native_remote_target,
};
use deve_core::utils::fs::{
    create_atomic_replace_temp, ensure_open_file_matches_path, lock_file_exclusive,
    open_regular_file_lock, replace_file_atomically, sync_directory,
};
use serde_json::Value;
use thiserror::Error;

const NATIVE_BACKEND_CONFIG_FILE: &str = "native-backend.json";
const NATIVE_BACKEND_LOCK_FILE: &str = ".native-backend.lock";
const REMOTE_PROBE_TIMEOUT: Duration = Duration::from_secs(5);
static NATIVE_BACKEND_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Error)]
pub enum DesktopNativeBackendError {
    #[error("desktop native backend config root unavailable: {0}")]
    ConfigRootUnavailable(String),
    #[error("desktop native backend config read failed")]
    ReadFailed(#[source] std::io::Error),
    #[error("desktop native backend config parse failed")]
    ParseFailed(#[source] serde_json::Error),
    #[error("desktop native backend config write failed")]
    WriteFailed(#[source] std::io::Error),
    #[error("desktop native backend config durability is uncertain; restart is required")]
    DurabilityUncertain,
    #[error("desktop native backend preference is invalid: {0}")]
    InvalidPreference(String),
    #[error("desktop remote backend probe failed: {0}")]
    ProbeFailed(String),
    #[error("desktop remote backend probe redirected away from requested origin")]
    ProbeRedirected,
    #[error("desktop remote backend returned invalid node role payload")]
    InvalidNodeRolePayload,
}

pub struct DesktopNativeBackendState {
    config_path: Option<PathBuf>,
    state: Mutex<DesktopNativeBackendRuntimeState>,
    unavailable_reason: Option<String>,
}

struct DesktopNativeBackendRuntimeState {
    preference: NativeBackendPreference,
    durability_uncertain: bool,
}

struct DesktopNativeBackendPersistFailure {
    error: std::io::Error,
    after_publish: bool,
}

impl DesktopNativeBackendState {
    pub fn from_data_root(data_root: Result<PathBuf, impl ToString>) -> Self {
        match data_root {
            Ok(data_root) => {
                let config_path = desktop_native_backend_config_path(&data_root);
                let preference = load_desktop_native_backend_preference_from_path(&config_path)
                    .unwrap_or_else(|error| {
                        eprintln!("desktop native backend preference ignored: {error}");
                        NativeBackendPreference::local()
                    });
                Self {
                    config_path: Some(config_path),
                    state: Mutex::new(DesktopNativeBackendRuntimeState {
                        preference,
                        durability_uncertain: false,
                    }),
                    unavailable_reason: None,
                }
            }
            Err(error) => Self {
                config_path: None,
                state: Mutex::new(DesktopNativeBackendRuntimeState {
                    preference: NativeBackendPreference::local(),
                    durability_uncertain: false,
                }),
                unavailable_reason: Some(error.to_string()),
            },
        }
    }

    pub fn preference(&self) -> Result<NativeBackendPreference, DesktopNativeBackendError> {
        let state = self.state.lock().map_err(|_| {
            DesktopNativeBackendError::ConfigRootUnavailable("state poisoned".into())
        })?;
        if state.durability_uncertain {
            return Err(DesktopNativeBackendError::DurabilityUncertain);
        }
        Ok(state.preference.clone())
    }

    pub(crate) fn save_preference(
        &self,
        preference: NativeBackendPreference,
    ) -> Result<(), DesktopNativeBackendError> {
        if let Some(reason) = self.unavailable_reason.as_ref() {
            return Err(DesktopNativeBackendError::ConfigRootUnavailable(
                reason.clone(),
            ));
        }
        let config_path = self.config_path.as_ref().ok_or_else(|| {
            DesktopNativeBackendError::ConfigRootUnavailable(
                "app-private config path is unavailable".into(),
            )
        })?;
        let preference = preference.canonicalized();
        native_shell_mode_for_backend_preference(&preference)
            .map_err(|error| DesktopNativeBackendError::InvalidPreference(error.to_string()))?;
        let mut state = self.state.lock().map_err(|_| {
            DesktopNativeBackendError::ConfigRootUnavailable("state poisoned".into())
        })?;
        if state.durability_uncertain {
            return Err(DesktopNativeBackendError::DurabilityUncertain);
        }
        match persist_desktop_native_backend_preference(config_path, &preference) {
            Ok(()) => {
                state.preference = preference;
                Ok(())
            }
            Err(failure) => {
                if failure.after_publish {
                    state.preference = preference;
                    state.durability_uncertain = true;
                }
                Err(DesktopNativeBackendError::WriteFailed(failure.error))
            }
        }
    }
}

pub fn desktop_native_backend_config_path(data_root: &Path) -> PathBuf {
    data_root.join(NATIVE_BACKEND_CONFIG_FILE)
}

pub fn load_desktop_native_backend_preference(
    data_root: &Path,
) -> Result<NativeBackendPreference, DesktopNativeBackendError> {
    load_desktop_native_backend_preference_from_path(&desktop_native_backend_config_path(data_root))
}

pub fn save_desktop_native_backend_preference(
    data_root: &Path,
    preference: &NativeBackendPreference,
) -> Result<(), DesktopNativeBackendError> {
    native_shell_mode_for_backend_preference(preference)
        .map_err(|error| DesktopNativeBackendError::InvalidPreference(error.to_string()))?;
    persist_desktop_native_backend_preference(
        &desktop_native_backend_config_path(data_root),
        preference,
    )
    .map_err(|failure| DesktopNativeBackendError::WriteFailed(failure.error))
}

fn load_desktop_native_backend_preference_from_path(
    config_path: &Path,
) -> Result<NativeBackendPreference, DesktopNativeBackendError> {
    if !config_path.exists() {
        return Ok(NativeBackendPreference::local());
    }
    let bytes = std::fs::read(config_path).map_err(DesktopNativeBackendError::ReadFailed)?;
    let preference: NativeBackendPreference =
        serde_json::from_slice(&bytes).map_err(DesktopNativeBackendError::ParseFailed)?;
    native_shell_mode_for_backend_preference(&preference)
        .map_err(|error| DesktopNativeBackendError::InvalidPreference(error.to_string()))?;
    Ok(match preference.mode {
        NativeBackendMode::Local => NativeBackendPreference::local(),
        NativeBackendMode::Remote => preference,
    })
}

fn persist_desktop_native_backend_preference(
    config_path: &Path,
    preference: &NativeBackendPreference,
) -> Result<(), DesktopNativeBackendPersistFailure> {
    native_shell_mode_for_backend_preference(preference).map_err(|error| {
        DesktopNativeBackendPersistFailure {
            error: std::io::Error::new(std::io::ErrorKind::InvalidInput, error.to_string()),
            after_publish: false,
        }
    })?;
    let preference = preference.canonicalized();
    let parent = config_path
        .parent()
        .ok_or_else(|| DesktopNativeBackendPersistFailure {
            error: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "desktop native backend config has no parent",
            ),
            after_publish: false,
        })?;
    std::fs::create_dir_all(parent).map_err(|error| DesktopNativeBackendPersistFailure {
        error,
        after_publish: false,
    })?;
    let lock_path = parent.join(NATIVE_BACKEND_LOCK_FILE);
    let lock = open_regular_file_lock(&lock_path, "desktop native backend config lock").map_err(
        |error| DesktopNativeBackendPersistFailure {
            error,
            after_publish: false,
        },
    )?;
    lock_file_exclusive(&lock).map_err(|error| DesktopNativeBackendPersistFailure {
        error,
        after_publish: false,
    })?;
    ensure_open_file_matches_path(&lock, &lock_path, "desktop native backend config lock")
        .map_err(|error| DesktopNativeBackendPersistFailure {
            error,
            after_publish: false,
        })?;
    let payload = serde_json::to_vec_pretty(&preference).map_err(|error| {
        DesktopNativeBackendPersistFailure {
            error: std::io::Error::new(std::io::ErrorKind::InvalidData, error),
            after_publish: false,
        }
    })?;
    let sequence = NATIVE_BACKEND_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp_path = config_path.with_file_name(format!(
        ".{NATIVE_BACKEND_CONFIG_FILE}.tmp-{}-{sequence}",
        std::process::id()
    ));
    let mut temp = create_atomic_replace_temp(&temp_path).map_err(|error| {
        DesktopNativeBackendPersistFailure {
            error,
            after_publish: false,
        }
    })?;
    let mut published = false;
    let result = (|| -> std::io::Result<()> {
        temp.write_all(&payload)?;
        temp.sync_all()?;
        replace_file_atomically(&temp, &temp_path, config_path)?;
        published = true;
        #[cfg(test)]
        if parent.join(".native-backend-fail-after-replace").exists() {
            return Err(std::io::Error::other(
                "injected desktop native backend failure after atomic replace",
            ));
        }
        sync_directory(parent)
    })();
    if result.is_err() && !published {
        let _ = std::fs::remove_file(&temp_path);
    }
    result.map_err(|error| DesktopNativeBackendPersistFailure {
        error,
        after_publish: published,
    })
}

pub fn normalized_native_remote_origin(
    remote_url: &str,
) -> Result<String, DesktopNativeBackendError> {
    let mut origin = remote_url.to_string();
    if origin.ends_with('/') {
        origin.pop();
    }
    validate_native_remote_target(&NativeRemoteTarget {
        https_origin: origin.clone(),
    })
    .map_err(|error| DesktopNativeBackendError::InvalidPreference(error.to_string()))?;
    Ok(origin)
}

pub async fn probe_desktop_native_remote_backend(
    remote_url: &str,
) -> NativeBackendValidationResult {
    match try_probe_desktop_native_remote_backend(remote_url).await {
        Ok(result) => result,
        Err(error) => NativeBackendValidationResult::failure(error.to_string()),
    }
}

async fn try_probe_desktop_native_remote_backend(
    remote_url: &str,
) -> Result<NativeBackendValidationResult, DesktopNativeBackendError> {
    let origin = normalized_native_remote_origin(remote_url)?;
    let client = reqwest::Client::builder()
        .timeout(REMOTE_PROBE_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| DesktopNativeBackendError::ProbeFailed(error.to_string()))?;
    let url = format!("{origin}/api/node/role");
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| DesktopNativeBackendError::ProbeFailed(error.to_string()))?;
    ensure_probe_response_origin(&origin, response.url())?;
    if !response.status().is_success() {
        return Err(DesktopNativeBackendError::ProbeFailed(format!(
            "HTTP {}",
            response.status()
        )));
    }
    let json = response
        .json::<Value>()
        .await
        .map_err(|error| DesktopNativeBackendError::ProbeFailed(error.to_string()))?;
    let node_role = json
        .get("role")
        .and_then(Value::as_str)
        .filter(|role| !role.trim().is_empty())
        .ok_or(DesktopNativeBackendError::InvalidNodeRolePayload)?;
    Ok(NativeBackendValidationResult::success(origin, node_role))
}

fn ensure_probe_response_origin(
    expected_origin: &str,
    response_url: &reqwest::Url,
) -> Result<(), DesktopNativeBackendError> {
    if response_url.origin().ascii_serialization() == expected_origin {
        Ok(())
    } else {
        Err(DesktopNativeBackendError::ProbeRedirected)
    }
}

#[cfg(test)]
mod tests;
