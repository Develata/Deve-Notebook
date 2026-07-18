//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract
//!   - 13_i18n#i18n-error-code-catalog
//!
//! Host-owned Remote Projection push failure taxonomy. Provider readiness is
//! separated from failures after upload execution begins so product callers
//! never infer a stable error code from diagnostic strings.

#[derive(Debug)]
pub(crate) enum ProjectionPushError {
    ProviderUnavailable(anyhow::Error),
    PushFailed(anyhow::Error),
}

impl ProjectionPushError {
    pub(crate) fn provider_unavailable(error: impl Into<anyhow::Error>) -> Self {
        Self::ProviderUnavailable(error.into())
    }

    pub(crate) fn push_failed(error: impl Into<anyhow::Error>) -> Self {
        Self::PushFailed(error.into())
    }

    pub(crate) fn is_provider_unavailable(&self) -> bool {
        matches!(self, Self::ProviderUnavailable(_))
    }
}

impl std::fmt::Display for ProjectionPushError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProviderUnavailable(error) => write!(
                formatter,
                "remote projection provider I/O did not complete (provider_io_ready=false): {error:#}"
            ),
            Self::PushFailed(error) => write!(
                formatter,
                "remote projection push failed (provider_io_ready=false): {error:#}"
            ),
        }
    }
}

impl std::error::Error for ProjectionPushError {}
