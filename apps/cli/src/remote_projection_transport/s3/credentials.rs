//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract

use deve_core::remote_projection::RemoteProjectionProviderError;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct S3Credentials {
    pub(super) access_key_id: String,
    pub(super) secret_access_key: String,
    pub(super) session_token: Option<String>,
}

impl std::fmt::Debug for S3Credentials {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("S3Credentials")
            .field("access_key_id", &"<redacted>")
            .field("secret_access_key", &"<redacted>")
            .field("session_token_present", &self.session_token.is_some())
            .finish()
    }
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

#[derive(Clone, PartialEq, Eq)]
pub(super) enum S3CredentialSource {
    Env,
    #[cfg(test)]
    Static(S3Credentials),
    #[cfg(test)]
    Fail(&'static str),
}

impl std::fmt::Debug for S3CredentialSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Env => formatter.write_str("Env"),
            #[cfg(test)]
            Self::Static(_) => formatter.write_str("Static(<redacted>)"),
            #[cfg(test)]
            Self::Fail(label) => formatter.debug_tuple("Fail").field(label).finish(),
        }
    }
}

impl S3CredentialSource {
    pub(super) fn resolve(&self) -> Result<S3Credentials, RemoteProjectionProviderError> {
        match self {
            Self::Env => credentials_from_env(),
            #[cfg(test)]
            Self::Static(credentials) => Ok(credentials.clone()),
            #[cfg(test)]
            Self::Fail(label) => Err(RemoteProjectionProviderError::ProviderIo(format!(
                "S3 credential source {label} should not resolve"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum S3RegionSource {
    Env,
    #[cfg(test)]
    Static(String),
    #[cfg(test)]
    Fail(&'static str),
}

impl S3RegionSource {
    pub(super) fn resolve(&self) -> Result<String, RemoteProjectionProviderError> {
        match self {
            Self::Env => region_from_env(),
            #[cfg(test)]
            Self::Static(region) => Ok(region.clone()),
            #[cfg(test)]
            Self::Fail(label) => Err(RemoteProjectionProviderError::ProviderIo(format!(
                "S3 region source {label} should not resolve"
            ))),
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
