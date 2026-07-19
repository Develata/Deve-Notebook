//! plan_ref:
//!   - 04_repository#host-repo-alias-contract
//!
//! Temporary pre-B1 adapter from exact RepoId membership to existing local
//! authority files. Shared catalog inputs fail globally; one broken repo is a
//! per-entry admission failure. B1 replaces this module with the durable catalog.

use super::model::HostRepoAliasError;
use crate::ledger::database_cache::reusable_cached_database;
use crate::ledger::manager::types::{RepoInfo, RepoManager};
use crate::models::RepoId;
use crate::utils::fs::open_regular_file_read;
use redb::{Database, DatabaseError};
use serde::Deserialize;
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

pub(super) const LEGACY_REMOVED_REPOS_FILE: &str = "removed-local-repos.toml";
const LEGACY_REMOVED_REPOS_VERSION: u32 = 1;
pub(super) const LEGACY_REMOVED_REPOS_MAX_BYTES: u64 = 1024 * 1024;

#[derive(Deserialize)]
struct LegacyRemovedLocalReposFile {
    version: u32,
    #[serde(default)]
    repo_ids: Vec<RepoId>,
}

pub(super) enum LocalRepoAdmission {
    Active,
    Unknown,
    Failed,
}

/// One operation-wide view of global catalog membership inputs.
pub(super) struct LocalRepoMembershipSnapshot {
    local_dir: PathBuf,
    removed: HashSet<RepoId>,
}

impl LocalRepoMembershipSnapshot {
    pub(super) fn load(ledger_dir: &Path) -> Result<Self, HostRepoAliasError> {
        let local_dir = checked_local_dir(ledger_dir, "loading host repo alias membership")?;
        let removed = load_legacy_removed_repo_ids(ledger_dir)?;
        Ok(Self { local_dir, removed })
    }

    pub(super) fn admit(&self, repo_id: RepoId) -> Result<LocalRepoAdmission, HostRepoAliasError> {
        if self.removed.contains(&repo_id) {
            return Ok(LocalRepoAdmission::Unknown);
        }
        let path = self.local_dir.join(format!("{repo_id}.redb"));
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(LocalRepoAdmission::Unknown);
            }
            Err(error) => return Err(error.into()),
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
            return Ok(LocalRepoAdmission::Failed);
        }
        let info = match read_local_repo_info_without_caching(&path)? {
            Some(info) => info,
            None => return Ok(LocalRepoAdmission::Failed),
        };
        Ok(if info.uuid == repo_id {
            LocalRepoAdmission::Active
        } else {
            LocalRepoAdmission::Failed
        })
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

fn load_legacy_removed_repo_ids(ledger_dir: &Path) -> Result<HashSet<RepoId>, HostRepoAliasError> {
    let path = crate::utils::notegit::host_dir(ledger_dir).join(LEGACY_REMOVED_REPOS_FILE);
    let file = match open_regular_file_read(&path, "legacy removed-repo registry") {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(HashSet::new()),
        Err(error) => return Err(error.into()),
    };
    let metadata = file.metadata()?;
    if metadata.len() > LEGACY_REMOVED_REPOS_MAX_BYTES {
        return Err(HostRepoAliasError::Runtime(anyhow::anyhow!(
            "legacy removed-repo registry exceeds {} bytes: {path:?}",
            LEGACY_REMOVED_REPOS_MAX_BYTES
        )));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(LEGACY_REMOVED_REPOS_MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LEGACY_REMOVED_REPOS_MAX_BYTES {
        return Err(HostRepoAliasError::Runtime(anyhow::anyhow!(
            "legacy removed-repo registry exceeded {} bytes while reading: {path:?}",
            LEGACY_REMOVED_REPOS_MAX_BYTES
        )));
    }
    let content =
        String::from_utf8(bytes).map_err(|error| HostRepoAliasError::Runtime(error.into()))?;
    let file: LegacyRemovedLocalReposFile =
        toml::from_str(&content).map_err(|error| HostRepoAliasError::Runtime(error.into()))?;
    if file.version != LEGACY_REMOVED_REPOS_VERSION {
        return Err(HostRepoAliasError::Runtime(anyhow::anyhow!(
            "unsupported legacy removed-repo registry version {}",
            file.version
        )));
    }
    Ok(file.repo_ids.into_iter().collect())
}

fn read_local_repo_info_without_caching(
    path: &Path,
) -> Result<Option<RepoInfo>, HostRepoAliasError> {
    if let Some(database) = reusable_cached_database(path)? {
        return Ok(RepoManager::read_local_repo_info_from_db(database.as_ref())
            .ok()
            .flatten());
    }
    let database = match Database::open(path) {
        Ok(database) => database,
        Err(DatabaseError::DatabaseAlreadyOpen) => {
            return Err(HostRepoAliasError::Runtime(anyhow::anyhow!(
                "local authority is already open by another process: {path:?}"
            )));
        }
        Err(_) => return Ok(None),
    };
    Ok(RepoManager::read_local_repo_info_from_db(&database)
        .ok()
        .flatten())
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
