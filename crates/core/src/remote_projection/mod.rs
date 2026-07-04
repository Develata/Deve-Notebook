//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!
//! Admission planning for Markdown projection remote transports.
//!
//! WebDAV/S3 projection sync is intentionally separate from encrypted backup
//! packs. This module validates transport intent and records the authority
//! boundary: push/pull acts only on the projection workspace, and pull must
//! enter ledger authority later through External Changes.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteProjectionProvider {
    WebDav,
    S3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteProjectionDirection {
    Push,
    Pull,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteProjectionPlanInput {
    pub provider: RemoteProjectionProvider,
    pub direction: RemoteProjectionDirection,
    pub locator: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteProjectionTransportPlan {
    pub provider: RemoteProjectionProvider,
    pub direction: RemoteProjectionDirection,
    pub locator: String,
    pub projection_scope: String,
    pub writes_ledger: bool,
    pub writes_git_main_mirror: bool,
    pub overwrites_projection_on_pull: bool,
    pub external_changes_confirmation_required: bool,
    pub provider_io_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RemoteProjectionError {
    #[error("remote projection locator is empty")]
    EmptyLocator,
    #[error("remote projection locator scheme does not match provider")]
    ProviderSchemeMismatch,
    #[error("remote projection locator must not contain credentials, query, or fragment data")]
    SecretMaterialForbidden,
    #[error("remote projection locator contains an unsafe path segment")]
    UnsafeRemotePath,
}

pub fn plan_remote_projection_transport(
    input: RemoteProjectionPlanInput,
) -> Result<RemoteProjectionTransportPlan, RemoteProjectionError> {
    let locator = validate_locator(input.provider, &input.locator)?.to_string();
    Ok(RemoteProjectionTransportPlan {
        provider: input.provider,
        direction: input.direction,
        locator,
        projection_scope: "markdown".into(),
        writes_ledger: false,
        writes_git_main_mirror: false,
        overwrites_projection_on_pull: input.direction == RemoteProjectionDirection::Pull,
        external_changes_confirmation_required: input.direction == RemoteProjectionDirection::Pull,
        provider_io_ready: false,
    })
}

fn validate_locator(
    provider: RemoteProjectionProvider,
    locator: &str,
) -> Result<&str, RemoteProjectionError> {
    let locator = locator.trim();
    if locator.is_empty() {
        return Err(RemoteProjectionError::EmptyLocator);
    }
    if locator.contains('?') || locator.contains('#') {
        return Err(RemoteProjectionError::SecretMaterialForbidden);
    }
    if locator_contains_credentials(locator) {
        return Err(RemoteProjectionError::SecretMaterialForbidden);
    }
    if !locator_scheme_matches(provider, locator) {
        return Err(RemoteProjectionError::ProviderSchemeMismatch);
    }
    if locator_has_unsafe_path_segment(locator) {
        return Err(RemoteProjectionError::UnsafeRemotePath);
    }
    Ok(locator)
}

fn locator_scheme_matches(provider: RemoteProjectionProvider, locator: &str) -> bool {
    match provider {
        RemoteProjectionProvider::WebDav => locator.starts_with("webdav+https://"),
        RemoteProjectionProvider::S3 => {
            locator.starts_with("s3://") || locator.starts_with("s3+https://")
        }
    }
}

fn locator_contains_credentials(locator: &str) -> bool {
    let Some(after_scheme) = locator.split_once("://").map(|(_, rest)| rest) else {
        return false;
    };
    let authority = after_scheme.split('/').next().unwrap_or_default();
    authority.contains('@')
}

fn locator_has_unsafe_path_segment(locator: &str) -> bool {
    let Some(after_scheme) = locator.split_once("://").map(|(_, rest)| rest) else {
        return true;
    };
    let mut parts = after_scheme.split('/').skip(1);
    parts.any(|segment| matches!(segment, "" | "." | ".."))
}

impl RemoteProjectionProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            RemoteProjectionProvider::WebDav => "webdav",
            RemoteProjectionProvider::S3 => "s3",
        }
    }
}

impl RemoteProjectionDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            RemoteProjectionDirection::Push => "push",
            RemoteProjectionDirection::Pull => "pull",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_plan_never_writes_ledger_or_git_mirror() {
        let plan = plan_remote_projection_transport(RemoteProjectionPlanInput {
            provider: RemoteProjectionProvider::WebDav,
            direction: RemoteProjectionDirection::Push,
            locator: "webdav+https://dav.example.com/notebooks/main".into(),
        })
        .expect("plan");

        assert_eq!(plan.projection_scope, "markdown");
        assert!(!plan.writes_ledger);
        assert!(!plan.writes_git_main_mirror);
        assert!(!plan.overwrites_projection_on_pull);
        assert!(!plan.external_changes_confirmation_required);
        assert!(!plan.provider_io_ready);
    }

    #[test]
    fn pull_plan_requires_external_changes_confirmation() {
        let plan = plan_remote_projection_transport(RemoteProjectionPlanInput {
            provider: RemoteProjectionProvider::S3,
            direction: RemoteProjectionDirection::Pull,
            locator: "s3://bucket/notebooks/main".into(),
        })
        .expect("plan");

        assert!(plan.overwrites_projection_on_pull);
        assert!(plan.external_changes_confirmation_required);
        assert!(!plan.writes_ledger);
        assert!(!plan.provider_io_ready);
    }

    #[test]
    fn normalizes_locator_before_returning_plan() {
        let plan = plan_remote_projection_transport(RemoteProjectionPlanInput {
            provider: RemoteProjectionProvider::S3,
            direction: RemoteProjectionDirection::Push,
            locator: "  s3://bucket/notebooks/main\n".into(),
        })
        .expect("plan");

        assert_eq!(plan.locator, "s3://bucket/notebooks/main");
    }

    #[test]
    fn rejects_wrong_scheme_or_secret_material() {
        assert_eq!(
            plan_remote_projection_transport(RemoteProjectionPlanInput {
                provider: RemoteProjectionProvider::WebDav,
                direction: RemoteProjectionDirection::Push,
                locator: "s3://bucket/notebooks".into(),
            })
            .expect_err("scheme"),
            RemoteProjectionError::ProviderSchemeMismatch
        );
        assert_eq!(
            plan_remote_projection_transport(RemoteProjectionPlanInput {
                provider: RemoteProjectionProvider::S3,
                direction: RemoteProjectionDirection::Push,
                locator: "s3://token@bucket/notebooks".into(),
            })
            .expect_err("secret"),
            RemoteProjectionError::SecretMaterialForbidden
        );
    }
}
