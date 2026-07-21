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
const PREPARED_IDENTITY_VERSION: u32 = 1;
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
    snapshot_from_db(manager, repo_id, lease.db())
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
    snapshot_from_db(manager, repo_id, authority.db()).map_err(|error| {
        RepoCatalogError::PreparedIdentityUnavailable {
            repo_id,
            detail: error.to_string(),
        }
    })
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
    let stem = repo_id.to_string();
    let repo_info = RepoManager::read_local_repo_info_from_db(db)?
        .ok_or_else(|| anyhow::anyhow!("repository metadata is missing for {repo_id}"))?;
    if repo_info.uuid != repo_id {
        anyhow::bail!(
            "prepared DB identity mismatch: expected {repo_id}, got {}",
            repo_info.uuid
        );
    }

    let locator =
        crate::ledger::manager::projection_locator::projection_locator_record_for_repo_id(
            ledger_dir, repo_id,
        )?
        .ok_or_else(|| anyhow::anyhow!("Projection Locator is missing for {repo_id}"))?;
    let workspace_segment = locator.workspace_segment.clone();
    let declared_workspace_root = locator.projection_base_abs.join(&workspace_segment);
    let workspace_root = std::fs::canonicalize(&declared_workspace_root).with_context(|| {
        format!(
            "Failed to canonicalize Projection workspace root: {:?}",
            declared_workspace_root
        )
    })?;
    notegit::validate_repo_identity_marker(&workspace_root, repo_id)?;

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
    };
    let bytes = serde_json::to_vec(&manifest)?;
    Ok(PreparedRepoIdentity::from_manifest_bytes(&bytes))
}
