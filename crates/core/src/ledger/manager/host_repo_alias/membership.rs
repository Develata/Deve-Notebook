//! plan_ref:
//!   - 04_repository#host-repo-alias-contract
//!   - 04_repository#repo-catalog-contract
//!
//! Alias admission is a pure projection of durable Normal catalog membership.
//! It deliberately does not inspect DB health, locator state, workspace paths,
//! watcher readiness, or the retired removed-repo registry.

use super::model::HostRepoAliasError;
use crate::ledger::manager::repo_catalog_runtime::normal_catalog_ids_for_ledger;
use crate::ledger::manager::types::RepoManager;
use crate::models::RepoId;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(super) enum LocalRepoAdmission {
    Active,
    Unknown,
}

pub(super) struct LocalRepoMembershipSnapshot {
    active: HashSet<RepoId>,
}

impl LocalRepoMembershipSnapshot {
    pub(super) fn load(ledger_dir: &Path) -> Result<Self, HostRepoAliasError> {
        let active = normal_catalog_ids_for_ledger(ledger_dir)
            .map_err(|error| HostRepoAliasError::Runtime(error.into()))?
            .into_iter()
            .collect();
        Ok(Self { active })
    }

    pub(super) fn admit(&self, repo_id: RepoId) -> Result<LocalRepoAdmission, HostRepoAliasError> {
        Ok(if self.active.contains(&repo_id) {
            LocalRepoAdmission::Active
        } else {
            LocalRepoAdmission::Unknown
        })
    }
}

pub(super) fn require_removed(
    ledger_dir: &Path,
    repo_id: RepoId,
) -> Result<(), HostRepoAliasError> {
    let snapshot =
        crate::ledger::manager::repo_catalog_runtime::catalog_bootstrap_snapshot_for_ledger(
            ledger_dir,
        )
        .map_err(|error| HostRepoAliasError::Runtime(error.into()))?;
    if snapshot.removed_repo_ids().contains(&repo_id) {
        Ok(())
    } else {
        Err(HostRepoAliasError::UnknownLocalRepo(repo_id))
    }
}

pub(super) fn checked_local_dir(
    ledger_dir: &Path,
    context: &str,
) -> Result<PathBuf, HostRepoAliasError> {
    let local_dir = RepoManager::checked_local_dir_for(ledger_dir, context)?;
    let metadata = std::fs::symlink_metadata(&local_dir)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
        return Err(HostRepoAliasError::StoreInvalid(format!(
            "local repo directory is not a regular directory: {local_dir:?}"
        )));
    }
    Ok(local_dir)
}

#[cfg(windows)]
fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse(_metadata: &std::fs::Metadata) -> bool {
    false
}
