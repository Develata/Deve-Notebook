//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 03_storage/index#repo-runtime-layout
//!
//! No-follow local authority resource opening and persistent owner locks.

use super::{LocalAuthorityError, RepoAuthorityResources};
use crate::ledger::RepoManager;
use crate::models::RepoId;
use crate::utils::fs::{
    create_regular_file_new, ensure_open_file_matches_path, open_regular_file_lock,
    open_regular_file_read,
};
use redb::Database;
use std::path::{Path, PathBuf};

const LOCK_DIRECTORY: &str = "repo-authority-locks";

impl RepoAuthorityResources {
    pub(super) fn db(&self) -> &Database {
        &self.db
    }
}

pub(super) fn open_resources(
    ledger_dir: &Path,
    repo_id: RepoId,
    create: bool,
) -> Result<RepoAuthorityResources, LocalAuthorityError> {
    let lock_dir = checked_lock_directory(ledger_dir)?;
    let lock_path = lock_dir.join(format!("{repo_id}.lock"));
    let authority_lock = open_regular_file_lock(&lock_path, "local authority lock")?;
    if let Err(error) = authority_lock.try_lock() {
        return match error {
            std::fs::TryLockError::WouldBlock => Err(LocalAuthorityError::Busy(repo_id)),
            std::fs::TryLockError::Error(error) => Err(LocalAuthorityError::Io(error)),
        };
    }
    ensure_open_file_matches_path(&authority_lock, &lock_path, "local authority lock")?;

    let local_dir = RepoManager::checked_local_dir_for(ledger_dir, "opening local authority")?;
    let db_path = local_dir.join(format!("{repo_id}.redb"));
    let db_witness = if create {
        if crate::utils::fs::checked_exists(&db_path, "new local authority database")? {
            return Err(LocalAuthorityError::Invariant(format!(
                "local authority path already exists for RepoId {repo_id}: {db_path:?}"
            )));
        }
        create_regular_file_new(&db_path, "new local authority database")?
    } else {
        open_regular_file_read(&db_path, "local authority database")?
    };
    let db = if create {
        Database::create(&db_path)?
    } else {
        Database::open(&db_path)?
    };
    Ok(RepoAuthorityResources {
        db,
        db_witness,
        authority_lock,
        lock_path,
        db_path,
    })
    .and_then(|resources| {
        validate_resource_identity(&resources)?;
        Ok(resources)
    })
}

pub(super) fn validate_resource_identity(
    resources: &RepoAuthorityResources,
) -> Result<(), LocalAuthorityError> {
    ensure_open_file_matches_path(
        &resources.authority_lock,
        &resources.lock_path,
        "local authority lock",
    )?;
    ensure_open_file_matches_path(
        &resources.db_witness,
        &resources.db_path,
        "local authority database",
    )?;
    Ok(())
}

pub(super) fn validate_existing(db_path: &Path, db: &Database) -> Result<(), LocalAuthorityError> {
    let stem = RepoManager::repo_stem_from_path(db_path, "validating local authority")?;
    RepoManager::validate_local_repo_execution_identity(db, &stem)?;
    RepoManager::validate_local_repo_schema(db)?;
    crate::ledger::runtime_tables::repair_client_op_index(db)?;
    crate::ledger::source_control::init_tables(db)?;
    Ok(())
}

fn checked_lock_directory(ledger_dir: &Path) -> Result<PathBuf, LocalAuthorityError> {
    let host = crate::utils::notegit::host_dir(ledger_dir);
    create_regular_directory(&host, "host runtime")?;
    let locks = host.join(LOCK_DIRECTORY);
    create_regular_directory(&locks, "local authority lock directory")?;
    Ok(locks)
}

pub(super) fn open_committed_cleanup_lock(
    ledger_dir: &Path,
    snapshot: &super::RepoAuthorityRemovalSnapshot,
) -> Result<(std::fs::File, PathBuf), LocalAuthorityError> {
    let repo_id = snapshot.repo_id();
    let lock_dir = checked_lock_directory(ledger_dir)?;
    let lock_path = lock_dir.join(format!("{repo_id}.lock"));
    let local_dir =
        RepoManager::checked_local_dir_for(ledger_dir, "resuming local authority cleanup")?;
    let db_path = local_dir.join(format!("{repo_id}.redb"));
    if snapshot.authority_lock().path() != lock_path || snapshot.database().path() != db_path {
        return Err(LocalAuthorityError::CleanupIdentityChanged(repo_id));
    }
    if snapshot.authority_lock().classify()? != crate::utils::fs::HostPathState::Exact {
        return Err(LocalAuthorityError::CleanupIdentityChanged(repo_id));
    }
    if snapshot.database().classify()? == crate::utils::fs::HostPathState::Changed {
        return Err(LocalAuthorityError::CleanupIdentityChanged(repo_id));
    }
    let authority_lock = open_regular_file_lock(&lock_path, "local authority cleanup lock")?;
    if let Err(error) = authority_lock.try_lock() {
        return match error {
            std::fs::TryLockError::WouldBlock => Err(LocalAuthorityError::Busy(repo_id)),
            std::fs::TryLockError::Error(error) => Err(LocalAuthorityError::Io(error)),
        };
    }
    ensure_open_file_matches_path(&authority_lock, &lock_path, "local authority cleanup lock")?;
    Ok((authority_lock, db_path))
}

fn create_regular_directory(path: &Path, context: &str) -> Result<(), LocalAuthorityError> {
    match std::fs::create_dir(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
        return Err(LocalAuthorityError::Invariant(format!(
            "{context} is not a regular directory: {path:?}"
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x0400 != 0
}

#[cfg(not(windows))]
fn is_reparse(_metadata: &std::fs::Metadata) -> bool {
    false
}
