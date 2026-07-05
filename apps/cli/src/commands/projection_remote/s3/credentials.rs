//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use deve_core::remote_projection::RemoteProjectionProviderError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct S3Credentials {
    pub(super) access_key_id: String,
    pub(super) secret_access_key: String,
    pub(super) session_token: Option<String>,
}

impl S3Credentials {
    #[cfg(test)]
    pub(super) fn for_test() -> Self {
        Self {
            access_key_id: "AKIDEXAMPLE".into(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".into(),
            session_token: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum S3CredentialSource {
    Env,
    #[cfg(test)]
    Static(S3Credentials),
}

impl S3CredentialSource {
    pub(super) fn resolve(&self) -> Result<S3Credentials, RemoteProjectionProviderError> {
        match self {
            Self::Env => credentials_from_env(),
            #[cfg(test)]
            Self::Static(credentials) => Ok(credentials.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum S3RegionSource {
    Env,
    #[cfg(test)]
    Static(String),
}

impl S3RegionSource {
    pub(super) fn resolve(&self) -> Result<String, RemoteProjectionProviderError> {
        match self {
            Self::Env => region_from_env(),
            #[cfg(test)]
            Self::Static(region) => Ok(region.clone()),
        }
    }
}

fn credentials_from_env() -> Result<S3Credentials, RemoteProjectionProviderError> {
    Ok(S3Credentials {
        access_key_id: required_env("AWS_ACCESS_KEY_ID")?,
        secret_access_key: required_env("AWS_SECRET_ACCESS_KEY")?,
        session_token: optional_env("AWS_SESSION_TOKEN"),
    })
}

fn region_from_env() -> Result<String, RemoteProjectionProviderError> {
    optional_env("AWS_REGION")
        .or_else(|| optional_env("AWS_DEFAULT_REGION"))
        .ok_or_else(|| {
            RemoteProjectionProviderError::ProviderIo(
                "S3 region is not configured; set AWS_REGION or AWS_DEFAULT_REGION".into(),
            )
        })
}

fn required_env(name: &str) -> Result<String, RemoteProjectionProviderError> {
    optional_env(name).ok_or_else(|| {
        RemoteProjectionProviderError::ProviderIo(format!(
            "S3 credential environment variable {name} is not configured"
        ))
    })
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
