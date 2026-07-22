//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!   - 03_storage/index#repo-runtime-layout
//!
//! No-follow local authority resource opening and persistent owner locks.

use super::{LocalAuthorityError, RepoAuthorityResources};
use crate::ledger::RepoManager;
use crate::models::RepoId;
use crate::utils::fs::{
    HostPathIdentity, HostPathKind, HostPathState, HostQuarantinePlan,
    create_regular_file_lock_new, create_regular_file_new, delete_pinned_identity,
    ensure_open_file_matches_identity, ensure_open_file_matches_path,
    open_regular_file_lock_existing, open_regular_file_read,
};
use redb::Database;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
    let lock_dir = checked_lock_directory(ledger_dir, create)?;
    let lock_path = authority_lock_path(ledger_dir, repo_id);
    debug_assert_eq!(lock_path.parent(), Some(lock_dir.as_path()));
    let authority_lock = if create {
        create_regular_file_lock_new(&lock_path, "local authority lock")?
    } else {
        open_regular_file_lock_existing(&lock_path, "local authority lock")?
    };
    if let Err(error) = authority_lock.try_lock() {
        return match error {
            std::fs::TryLockError::WouldBlock => Err(LocalAuthorityError::Busy(repo_id)),
            std::fs::TryLockError::Error(error) => Err(LocalAuthorityError::Io(error)),
        };
    }
    ensure_open_file_matches_path(&authority_lock, &lock_path, "local authority lock")?;
    let authority_lock = Arc::new(authority_lock);

    let local_dir = RepoManager::checked_local_dir_for(ledger_dir, "opening local authority")?;
    let db_path = database_path(ledger_dir, repo_id);
    debug_assert_eq!(db_path.parent(), Some(local_dir.as_path()));
    let (db, db_witness) = if create {
        if crate::utils::fs::checked_exists(&db_path, "new local authority database")? {
            return Err(LocalAuthorityError::Invariant(format!(
                "local authority path already exists for RepoId {repo_id}: {db_path:?}"
            )));
        }
        let db_file = create_regular_file_new(&db_path, "new local authority database")?;
        let db_witness = db_file.try_clone()?;
        (Database::builder().create_file(db_file)?, db_witness)
    } else {
        // `Database::open` is intentionally retained here. Redb's
        // `Builder::create_file` is initialization-capable and therefore must
        // never be used for ordinary existing admission.
        let db_witness = open_regular_file_read(&db_path, "local authority database")?;
        (Database::open(&db_path)?, db_witness)
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

pub(super) fn open_reopening_lock(
    ledger_dir: &Path,
    repo_id: RepoId,
    expected_lock: &HostPathIdentity,
    removed_database: &HostPathIdentity,
) -> Result<Arc<std::fs::File>, LocalAuthorityError> {
    let lock_dir = checked_lock_directory(ledger_dir, false)?;
    let lock_path = authority_lock_path(ledger_dir, repo_id);
    let db_path = database_path(ledger_dir, repo_id);
    if lock_path.parent() != Some(lock_dir.as_path())
        || expected_lock.path() != lock_path
        || expected_lock.kind() != HostPathKind::RegularFile
        || removed_database.path() != db_path
        || removed_database.kind() != HostPathKind::RegularFile
        || removed_database.classify()? != HostPathState::Missing
    {
        return Err(LocalAuthorityError::RepairRequired(repo_id));
    }

    let authority_lock =
        open_regular_file_lock_existing(&lock_path, "retired local authority lock")?;
    if let Err(error) = authority_lock.try_lock() {
        return match error {
            std::fs::TryLockError::WouldBlock => Err(LocalAuthorityError::Busy(repo_id)),
            std::fs::TryLockError::Error(error) => Err(LocalAuthorityError::Io(error)),
        };
    }
    ensure_open_file_matches_identity(
        &authority_lock,
        expected_lock,
        "retired local authority lock",
    )?;
    if removed_database.classify()? != HostPathState::Missing {
        return Err(LocalAuthorityError::RepairRequired(repo_id));
    }
    Ok(Arc::new(authority_lock))
}

pub(super) fn create_reopening_resources(
    ledger_dir: &Path,
    repo_id: RepoId,
    expected_lock: &HostPathIdentity,
    removed_database: &HostPathIdentity,
    authority_lock: Arc<std::fs::File>,
) -> Result<RepoAuthorityResources, LocalAuthorityError> {
    let lock_path = authority_lock_path(ledger_dir, repo_id);
    let db_path = database_path(ledger_dir, repo_id);
    ensure_open_file_matches_identity(
        &authority_lock,
        expected_lock,
        "retired local authority lock",
    )?;
    if removed_database.path() != db_path
        || removed_database.kind() != HostPathKind::RegularFile
        || removed_database.classify()? != HostPathState::Missing
    {
        return Err(LocalAuthorityError::RepairRequired(repo_id));
    }

    // Create and initialize through an exact handle, but publish the file at
    // the canonical pathname only through the hardened same-parent,
    // no-replace primitive. This binds the new incarnation to the parent
    // identity frozen by the removed database manifest.
    let parent = db_path.parent().ok_or_else(|| {
        LocalAuthorityError::Invariant("local authority database has no parent".to_string())
    })?;
    let temp_path = parent.join(format!(
        ".deve-reopening-{}-{repo_id}.redb.tmp",
        uuid::Uuid::new_v4().simple()
    ));
    let db_file = create_regular_file_new(&temp_path, "reopening local authority database")?;
    let temp_identity = HostPathIdentity::capture(&temp_path, HostPathKind::RegularFile)?;
    if temp_identity.parent_identity() != removed_database.parent_identity() {
        let _ = delete_pinned_identity(&temp_identity);
        return Err(LocalAuthorityError::RepairRequired(repo_id));
    }
    let publish = match HostQuarantinePlan::same_parent(temp_identity.clone(), db_path.clone()) {
        Ok(plan) => plan,
        Err(error) => {
            let _ = delete_pinned_identity(&temp_identity);
            return Err(error.into());
        }
    };
    if removed_database.classify()? != HostPathState::Missing {
        let _ = delete_pinned_identity(&temp_identity);
        return Err(LocalAuthorityError::RepairRequired(repo_id));
    }
    let cut = publish.cut()?;
    if cut.path() != db_path || !cut.original_path_is_absent()? {
        return Err(LocalAuthorityError::RepairRequired(repo_id));
    }
    let database_identity = HostPathIdentity::capture(&db_path, HostPathKind::RegularFile)?;
    if database_identity.parent_identity() != removed_database.parent_identity()
        || database_identity.object_identity() != temp_identity.object_identity()
    {
        return Err(LocalAuthorityError::RepairRequired(repo_id));
    }
    ensure_open_file_matches_identity(
        &db_file,
        &database_identity,
        "reopened local authority database",
    )?;

    let db_witness = db_file.try_clone()?;
    let db = Database::builder().create_file(db_file)?;
    let resources = RepoAuthorityResources {
        db,
        db_witness,
        authority_lock,
        lock_path,
        db_path,
    };
    validate_resource_identity(&resources)?;
    ensure_open_file_matches_identity(&resources.authority_lock, expected_lock, "retired lock")?;
    Ok(resources)
}

pub(super) fn authority_lock_path(ledger_dir: &Path, repo_id: RepoId) -> PathBuf {
    crate::utils::notegit::host_dir(ledger_dir)
        .join(LOCK_DIRECTORY)
        .join(format!("{repo_id}.lock"))
}

pub(super) fn database_path(ledger_dir: &Path, repo_id: RepoId) -> PathBuf {
    ledger_dir.join("local").join(format!("{repo_id}.redb"))
}

fn checked_lock_directory(ledger_dir: &Path, create: bool) -> Result<PathBuf, LocalAuthorityError> {
    let host = crate::utils::notegit::host_dir(ledger_dir);
    ensure_regular_directory(&host, "host runtime", create)?;
    let locks = host.join(LOCK_DIRECTORY);
    ensure_regular_directory(&locks, "local authority lock directory", create)?;
    Ok(locks)
}

pub(super) fn open_committed_cleanup_lock(
    ledger_dir: &Path,
    snapshot: &super::RepoAuthorityRemovalSnapshot,
) -> Result<(std::fs::File, PathBuf), LocalAuthorityError> {
    let repo_id = snapshot.repo_id();
    let lock_dir = checked_lock_directory(ledger_dir, false)?;
    let lock_path = lock_dir.join(format!("{repo_id}.lock"));
    let local_dir =
        RepoManager::checked_local_dir_for(ledger_dir, "resuming local authority cleanup")?;
    let db_path = local_dir.join(format!("{repo_id}.redb"));
    if snapshot.authority_lock().path() != lock_path || snapshot.database().path() != db_path {
        return Err(LocalAuthorityError::CleanupIdentityChanged(repo_id));
    }
    if snapshot.authority_lock().classify()? != HostPathState::Exact {
        return Err(LocalAuthorityError::CleanupIdentityChanged(repo_id));
    }
    if snapshot.database().classify()? == HostPathState::Changed {
        return Err(LocalAuthorityError::CleanupIdentityChanged(repo_id));
    }
    let authority_lock =
        open_regular_file_lock_existing(&lock_path, "local authority cleanup lock")?;
    if let Err(error) = authority_lock.try_lock() {
        return match error {
            std::fs::TryLockError::WouldBlock => Err(LocalAuthorityError::Busy(repo_id)),
            std::fs::TryLockError::Error(error) => Err(LocalAuthorityError::Io(error)),
        };
    }
    ensure_open_file_matches_identity(
        &authority_lock,
        snapshot.authority_lock(),
        "local authority cleanup lock",
    )?;
    Ok((authority_lock, db_path))
}

fn ensure_regular_directory(
    path: &Path,
    context: &str,
    create: bool,
) -> Result<(), LocalAuthorityError> {
    if create {
        match std::fs::create_dir(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
    }
    if HostPathIdentity::capture(path, HostPathKind::Directory).is_err() {
        return Err(LocalAuthorityError::Invariant(format!(
            "{context} is not a regular directory: {path:?}"
        )));
    }
    Ok(())
}
