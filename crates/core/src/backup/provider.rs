//! plan_ref:
//!   - 18_backup#backup-provider-dispatch-contract
//!
//! Backup provider adapter dispatch.
//!
//! This module maps a parsed locator plus secret references to a provider
//! adapter plan. It does not open sockets, issue WebDAV/S3 requests, resolve
//! credentials, read key material, or treat provider metadata as authority.

use super::locator::{BackupLocator, BackupProviderKind};
use super::secret::{BackupSecretRef, BackupSecretRefKind};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupProviderDispatchInput {
    pub locator: BackupLocator,
    pub credential_ref: BackupSecretRef,
    pub key_ref: BackupSecretRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupProviderAdapterPlan {
    pub provider: BackupProviderKind,
    pub endpoint: Option<String>,
    pub namespace: String,
    pub repo_root_path: String,
    pub credential_ref: BackupSecretRef,
    pub key_ref: BackupSecretRef,
    pub supports_remote_listing: bool,
    pub provider_metadata_is_diagnostic_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupProviderDispatchError {
    #[error("backup provider endpoint is required for this adapter")]
    MissingEndpoint,
    #[error("backup provider endpoint is not allowed for this adapter")]
    EndpointForbidden,
    #[error("backup provider endpoint must use https")]
    NonHttpsEndpoint,
    #[error("backup provider credential ref has the wrong kind")]
    CredentialRefKindMismatch,
    #[error("backup provider key ref has the wrong kind")]
    KeyRefKindMismatch,
}

pub fn dispatch_backup_provider_adapter(
    input: BackupProviderDispatchInput,
) -> Result<BackupProviderAdapterPlan, BackupProviderDispatchError> {
    validate_secret_ref_kinds(&input.credential_ref, &input.key_ref)?;
    validate_endpoint(input.locator.provider, input.locator.endpoint.as_deref())?;

    Ok(BackupProviderAdapterPlan {
        provider: input.locator.provider,
        endpoint: input.locator.endpoint,
        namespace: input.locator.namespace,
        repo_root_path: input.locator.repo_root_path,
        credential_ref: input.credential_ref,
        key_ref: input.key_ref,
        supports_remote_listing: true,
        provider_metadata_is_diagnostic_only: true,
    })
}

fn validate_secret_ref_kinds(
    credential_ref: &BackupSecretRef,
    key_ref: &BackupSecretRef,
) -> Result<(), BackupProviderDispatchError> {
    if credential_ref.kind != BackupSecretRefKind::Credential {
        return Err(BackupProviderDispatchError::CredentialRefKindMismatch);
    }
    if key_ref.kind != BackupSecretRefKind::Key {
        return Err(BackupProviderDispatchError::KeyRefKindMismatch);
    }
    Ok(())
}

fn validate_endpoint(
    provider: BackupProviderKind,
    endpoint: Option<&str>,
) -> Result<(), BackupProviderDispatchError> {
    match provider {
        BackupProviderKind::WebDavHttps | BackupProviderKind::S3CompatibleHttps => {
            let Some(endpoint) = endpoint else {
                return Err(BackupProviderDispatchError::MissingEndpoint);
            };
            if !endpoint.starts_with("https://") {
                return Err(BackupProviderDispatchError::NonHttpsEndpoint);
            }
            Ok(())
        }
        BackupProviderKind::S3 => {
            if endpoint.is_some() {
                return Err(BackupProviderDispatchError::EndpointForbidden);
            }
            Ok(())
        }
    }
}
