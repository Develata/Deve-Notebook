//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 03_storage/projection#projection-locator-contract

use super::{PreparedRepoIdentity, RepoCatalogError};
use crate::ledger::PreparedRepoAuthority;
use crate::ledger::manager::types::RepoManager;
use crate::models::RepoId;
use crate::utils::notegit;
use crate::utils::path::path_to_forward_slash;
use anyhow::Context;
use serde::Serialize;

const PREPARED_IDENTITY_FORMAT: &str = "deve.prepared-repo-identity";
const PREPARED_IDENTITY_VERSION: u32 = 2;
#[derive(Serialize)]
struct PreparedIdentityManifest<'a> {
    format: &'static str,
    version: u32,
    repo_id: RepoId,
    db_stem: &'a str,
    repo_url: &'a Option<String>,
    projection_base: String,
    workspace_segment: &'a str,
    locator_binding_epoch: i64,
    workspace_marker_repo_id: RepoId,
    authority_database: &'a crate::utils::fs::HostPathIdentity,
    authority_lock: &'a crate::utils::fs::HostPathIdentity,
    locator_store: &'a crate::utils::fs::HostPathIdentity,
    workspace_root: &'a crate::utils::fs::HostPathIdentity,
    workspace_marker: &'a crate::utils::fs::HostPathIdentity,
}

pub(super) fn snapshot(
    manager: &RepoManager,
    repo_id: RepoId,
) -> Result<PreparedRepoIdentity, RepoCatalogError> {
    snapshot_inner(manager, repo_id).map_err(|error| {
        RepoCatalogError::PreparedIdentityUnavailable {
            repo_id,
            detail: error.to_string(),
        }
    })
}

pub(super) fn snapshot_initial_primary(
    manager: &RepoManager,
    repo_id: RepoId,
) -> Result<PreparedRepoIdentity, RepoCatalogError> {
    manager
        .with_initial_primary_for_catalog(repo_id, |db| {
            snapshot_from_db(manager, repo_id, db)
                .map_err(crate::ledger::LocalAuthorityError::Other)
        })
        .map_err(|error| RepoCatalogError::PreparedIdentityUnavailable {
            repo_id,
            detail: error.to_string(),
        })
}

fn snapshot_inner(manager: &RepoManager, repo_id: RepoId) -> anyhow::Result<PreparedRepoIdentity> {
    let lease = manager.lease_local_authority(repo_id)?;
    let authority = lease.identity_observation()?;
    let locator =
        crate::ledger::manager::projection_locator::ProjectionLocatorActivationGuard::acquire(
            &manager.ledger_dir,
            repo_id,
        )?;
    snapshot_from_observations(repo_id, lease.db(), &authority, &locator)
}

pub(super) fn snapshot_prepared(
    manager: &RepoManager,
    repo_id: RepoId,
    authority: &PreparedRepoAuthority,
) -> Result<PreparedRepoIdentity, RepoCatalogError> {
    if authority.repo_id() != repo_id {
        return Err(RepoCatalogError::PreparedIdentityUnavailable {
            repo_id,
            detail: format!(
                "prepared authority RepoId mismatch: expected {repo_id}, got {}",
                authority.repo_id()
            ),
        });
    }
    let result = (|| {
        let authority_identity = authority.identity_observation()?;
        let locator =
            crate::ledger::manager::projection_locator::ProjectionLocatorActivationGuard::acquire(
                &manager.ledger_dir,
                repo_id,
            )?;
        snapshot_from_observations(repo_id, authority.db(), &authority_identity, &locator)
    })();
    result.map_err(
        |error: anyhow::Error| RepoCatalogError::PreparedIdentityUnavailable {
            repo_id,
            detail: error.to_string(),
        },
    )
}

fn snapshot_from_db(
    manager: &RepoManager,
    repo_id: RepoId,
    db: &redb::Database,
) -> anyhow::Result<PreparedRepoIdentity> {
    snapshot_from_db_at(&manager.ledger_dir, repo_id, db)
}

pub(super) fn snapshot_from_db_at(
    ledger_dir: &std::path::Path,
    repo_id: RepoId,
    db: &redb::Database,
) -> anyhow::Result<PreparedRepoIdentity> {
    let database_path = ledger_dir.join("local").join(format!("{repo_id}.redb"));
    let lock_path = crate::utils::notegit::host_dir(ledger_dir)
        .join("repo-authority-locks")
        .join(format!("{repo_id}.lock"));
    let authority = crate::ledger::manager::authority_storage_runtime::PreparedAuthorityIdentity {
        database: crate::utils::fs::HostPathIdentity::capture(
            &database_path,
            crate::utils::fs::HostPathKind::RegularFile,
        )?,
        authority_lock: crate::utils::fs::HostPathIdentity::capture(
            &lock_path,
            crate::utils::fs::HostPathKind::RegularFile,
        )?,
    };
    let locator =
        crate::ledger::manager::projection_locator::ProjectionLocatorActivationGuard::acquire(
            ledger_dir, repo_id,
        )?;
    snapshot_from_observations(repo_id, db, &authority, &locator)
}

pub(crate) fn snapshot_prepared_with_guard(
    repo_id: RepoId,
    authority: &PreparedRepoAuthority,
    locator: &crate::ledger::manager::projection_locator::ProjectionLocatorActivationGuard,
) -> anyhow::Result<(
    PreparedRepoIdentity,
    crate::ledger::manager::authority_storage_runtime::PreparedAuthorityIdentity,
)> {
    let authority_identity = authority.identity_observation()?;
    let identity =
        snapshot_from_observations(repo_id, authority.db(), &authority_identity, locator)?;
    Ok((identity, authority_identity))
}

fn snapshot_from_observations(
    repo_id: RepoId,
    db: &redb::Database,
    authority: &crate::ledger::manager::authority_storage_runtime::PreparedAuthorityIdentity,
    locator_guard: &crate::ledger::manager::projection_locator::ProjectionLocatorActivationGuard,
) -> anyhow::Result<PreparedRepoIdentity> {
    locator_guard.revalidate()?;
    let stem = repo_id.to_string();
    let repo_info = RepoManager::read_local_repo_info_from_db(db)?
        .ok_or_else(|| anyhow::anyhow!("repository metadata is missing for {repo_id}"))?;
    if repo_info.uuid != repo_id {
        anyhow::bail!(
            "prepared DB identity mismatch: expected {repo_id}, got {}",
            repo_info.uuid
        );
    }

    let locator = locator_guard.locator();
    let workspace_segment = locator.workspace_segment.clone();
    let declared_workspace_root = locator.projection_base_abs.join(&workspace_segment);
    let workspace_root = std::fs::canonicalize(&declared_workspace_root).with_context(|| {
        format!(
            "Failed to canonicalize Projection workspace root: {:?}",
            declared_workspace_root
        )
    })?;
    notegit::validate_repo_identity_marker(&workspace_root, repo_id)?;
    if workspace_root != locator_guard.workspace_root().path() {
        anyhow::bail!("prepared workspace root changed for {repo_id}");
    }

    let manifest = PreparedIdentityManifest {
        format: PREPARED_IDENTITY_FORMAT,
        version: PREPARED_IDENTITY_VERSION,
        repo_id,
        db_stem: &stem,
        repo_url: &repo_info.url,
        projection_base: path_to_forward_slash(&locator.projection_base_abs),
        workspace_segment: &workspace_segment,
        locator_binding_epoch: locator.canonicalized_at_unix_ms,
        workspace_marker_repo_id: repo_id,
        authority_database: authority.database(),
        authority_lock: authority.authority_lock(),
        locator_store: locator_guard.store(),
        workspace_root: locator_guard.workspace_root(),
        workspace_marker: locator_guard.marker(),
    };
    let bytes = serde_json::to_vec(&manifest)?;
    Ok(PreparedRepoIdentity::from_manifest_bytes(&bytes))
}
