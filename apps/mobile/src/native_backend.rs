//! plan_ref:
//!   - 11_ui_design/index#native-post-gate-common-contract
//!   - 15_settings#native-host-local-backend-preference

use std::{
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use deve_core::native_adapter::{
    NativeBackendMode, NativeBackendPreference, NativeBackendValidationResult, NativeRemoteTarget,
    native_shell_mode_for_backend_preference, validate_native_remote_target,
};
use serde_json::Value;
use thiserror::Error;

const NATIVE_BACKEND_CONFIG_FILE: &str = "native-backend.json";
const REMOTE_PROBE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Error)]
pub enum MobileNativeBackendError {
    #[error("mobile native backend config root unavailable: {0}")]
    ConfigRootUnavailable(String),
    #[error("mobile native backend config read failed")]
    ReadFailed(#[source] std::io::Error),
    #[error("mobile native backend config parse failed")]
    ParseFailed(#[source] serde_json::Error),
    #[error("mobile native backend config write failed")]
    WriteFailed(#[source] std::io::Error),
    #[error("mobile native backend preference is invalid: {0}")]
    InvalidPreference(String),
    #[error("mobile remote backend probe failed: {0}")]
    ProbeFailed(String),
    #[error("mobile remote backend probe redirected away from requested origin")]
    ProbeRedirected,
    #[error("mobile remote backend returned invalid node role payload")]
    InvalidNodeRolePayload,
}

pub struct MobileNativeBackendState {
    config_path: Option<PathBuf>,
    preference: Mutex<NativeBackendPreference>,
    unavailable_reason: Option<String>,
}

impl MobileNativeBackendState {
    pub fn from_data_root(data_root: Result<PathBuf, impl ToString>) -> Self {
        match data_root {
            Ok(data_root) => {
                let config_path = mobile_native_backend_config_path(&data_root);
                let preference = load_mobile_native_backend_preference_from_path(&config_path)
                    .unwrap_or_else(|error| {
                        eprintln!("mobile native backend preference ignored: {error}");
                        NativeBackendPreference::local()
                    });
                Self {
                    config_path: Some(config_path),
                    preference: Mutex::new(preference),
                    unavailable_reason: None,
                }
            }
            Err(error) => Self {
                config_path: None,
                preference: Mutex::new(NativeBackendPreference::local()),
                unavailable_reason: Some(error.to_string()),
            },
        }
    }

    pub fn preference(&self) -> Result<NativeBackendPreference, MobileNativeBackendError> {
        self.preference
            .lock()
            .map(|preference| preference.clone())
            .map_err(|_| MobileNativeBackendError::ConfigRootUnavailable("state poisoned".into()))
    }

    pub(crate) fn save_preference(
        &self,
        preference: NativeBackendPreference,
    ) -> Result<(), MobileNativeBackendError> {
        if let Some(reason) = self.unavailable_reason.as_ref() {
            return Err(MobileNativeBackendError::ConfigRootUnavailable(
                reason.clone(),
            ));
        }
        let config_path = self.config_path.as_ref().ok_or_else(|| {
            MobileNativeBackendError::ConfigRootUnavailable(
                "app-private config path is unavailable".into(),
            )
        })?;
        save_mobile_native_backend_preference_to_path(config_path, &preference)?;
        let mut current = self.preference.lock().map_err(|_| {
            MobileNativeBackendError::ConfigRootUnavailable("state poisoned".into())
        })?;
        *current = preference;
        Ok(())
    }
}

pub fn mobile_native_backend_config_path(data_root: &Path) -> PathBuf {
    data_root.join(NATIVE_BACKEND_CONFIG_FILE)
}

pub fn load_mobile_native_backend_preference(
    data_root: &Path,
) -> Result<NativeBackendPreference, MobileNativeBackendError> {
    load_mobile_native_backend_preference_from_path(&mobile_native_backend_config_path(data_root))
}

pub fn save_mobile_native_backend_preference(
    data_root: &Path,
    preference: &NativeBackendPreference,
) -> Result<(), MobileNativeBackendError> {
    save_mobile_native_backend_preference_to_path(
        &mobile_native_backend_config_path(data_root),
        preference,
    )
}

fn load_mobile_native_backend_preference_from_path(
    config_path: &Path,
) -> Result<NativeBackendPreference, MobileNativeBackendError> {
    if !config_path.exists() {
        return Ok(NativeBackendPreference::local());
    }
    let bytes = std::fs::read(config_path).map_err(MobileNativeBackendError::ReadFailed)?;
    let preference: NativeBackendPreference =
        serde_json::from_slice(&bytes).map_err(MobileNativeBackendError::ParseFailed)?;
    native_shell_mode_for_backend_preference(&preference)
        .map_err(|error| MobileNativeBackendError::InvalidPreference(error.to_string()))?;
    Ok(match preference.mode {
        NativeBackendMode::Local => NativeBackendPreference::local(),
        NativeBackendMode::Remote => preference,
    })
}

fn save_mobile_native_backend_preference_to_path(
    config_path: &Path,
    preference: &NativeBackendPreference,
) -> Result<(), MobileNativeBackendError> {
    native_shell_mode_for_backend_preference(preference)
        .map_err(|error| MobileNativeBackendError::InvalidPreference(error.to_string()))?;
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(MobileNativeBackendError::WriteFailed)?;
    }
    let payload = serde_json::to_vec_pretty(preference).map_err(|error| {
        MobileNativeBackendError::WriteFailed(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error,
        ))
    })?;
    let temp_path = config_path.with_extension("json.tmp");
    std::fs::write(&temp_path, payload).map_err(MobileNativeBackendError::WriteFailed)?;
    if config_path.exists() {
        std::fs::remove_file(config_path).map_err(MobileNativeBackendError::WriteFailed)?;
    }
    std::fs::rename(temp_path, config_path).map_err(MobileNativeBackendError::WriteFailed)
}

pub fn normalized_native_remote_origin(
    remote_url: &str,
) -> Result<String, MobileNativeBackendError> {
    let mut origin = remote_url.to_string();
    if origin.ends_with('/') {
        origin.pop();
    }
    validate_native_remote_target(&NativeRemoteTarget {
        https_origin: origin.clone(),
    })
    .map_err(|error| MobileNativeBackendError::InvalidPreference(error.to_string()))?;
    Ok(origin)
}

pub async fn probe_mobile_native_remote_backend(remote_url: &str) -> NativeBackendValidationResult {
    match try_probe_mobile_native_remote_backend(remote_url).await {
        Ok(result) => result,
        Err(error) => NativeBackendValidationResult::failure(error.to_string()),
    }
}

async fn try_probe_mobile_native_remote_backend(
    remote_url: &str,
) -> Result<NativeBackendValidationResult, MobileNativeBackendError> {
    let origin = normalized_native_remote_origin(remote_url)?;
    let client = reqwest::Client::builder()
        .timeout(REMOTE_PROBE_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| MobileNativeBackendError::ProbeFailed(error.to_string()))?;
    let url = format!("{origin}/api/node/role");
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| MobileNativeBackendError::ProbeFailed(error.to_string()))?;
    ensure_probe_response_origin(&origin, response.url())?;
    if !response.status().is_success() {
        return Err(MobileNativeBackendError::ProbeFailed(format!(
            "HTTP {}",
            response.status()
        )));
    }
    let json = response
        .json::<Value>()
        .await
        .map_err(|error| MobileNativeBackendError::ProbeFailed(error.to_string()))?;
    let node_role = json
        .get("role")
        .and_then(Value::as_str)
        .filter(|role| !role.trim().is_empty())
        .ok_or(MobileNativeBackendError::InvalidNodeRolePayload)?;
    Ok(NativeBackendValidationResult::success(origin, node_role))
}

fn ensure_probe_response_origin(
    expected_origin: &str,
    response_url: &reqwest::Url,
) -> Result<(), MobileNativeBackendError> {
    if response_url.origin().ascii_serialization() == expected_origin {
        Ok(())
    } else {
        Err(MobileNativeBackendError::ProbeRedirected)
    }
}

#[cfg(test)]
mod tests;
