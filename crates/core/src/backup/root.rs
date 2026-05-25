//! plan_ref:
//!   - 06_backup#backup-root-contract
//!
//! Backup root manifest validation.
//!
//! Backup root state describes a remote namespace for a logical repo. It is a
//! discovery and verification boundary only; it does not become repo identity,
//! ledger authority, source-control authority, or provider state.

use super::locator::{BackupLocator, BackupProviderKind};
use crate::models::RepoId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(test)]
mod tests;

pub const BACKUP_ROOT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupRootInput {
    pub repo_locator: BackupLocator,
    pub expected_repo_id: RepoId,
    pub manifest_repo_id: RepoId,
    pub format_version: u32,
    pub provider_kind: BackupProviderKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupRoot {
    pub repo_locator: BackupLocator,
    pub repo_id: RepoId,
    pub format_version: u32,
    pub provider_kind: BackupProviderKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BackupRootError {
    #[error("backup root format version is unsupported: {0}")]
    UnsupportedFormatVersion(u32),
    #[error("backup root manifest repo id does not match expected repo")]
    RepoIdMismatch,
    #[error("backup root provider kind does not match locator provider")]
    ProviderKindMismatch,
}

pub fn plan_backup_root(input: BackupRootInput) -> Result<BackupRoot, BackupRootError> {
    if input.format_version != BACKUP_ROOT_FORMAT_VERSION {
        return Err(BackupRootError::UnsupportedFormatVersion(
            input.format_version,
        ));
    }
    if input.manifest_repo_id != input.expected_repo_id {
        return Err(BackupRootError::RepoIdMismatch);
    }
    if input.provider_kind != input.repo_locator.provider {
        return Err(BackupRootError::ProviderKindMismatch);
    }

    Ok(BackupRoot {
        repo_locator: input.repo_locator,
        repo_id: input.expected_repo_id,
        format_version: BACKUP_ROOT_FORMAT_VERSION,
        provider_kind: input.provider_kind,
    })
}
