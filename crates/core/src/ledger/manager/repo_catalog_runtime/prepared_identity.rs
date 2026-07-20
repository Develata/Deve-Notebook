//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 03_storage/projection#projection-locator-contract

use super::{PreparedRepoIdentity, RepoCatalogError};
use crate::ledger::manager::types::RepoManager;
use crate::models::RepoId;
use crate::utils::notegit;
use crate::utils::path::path_to_forward_slash;
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

fn snapshot_inner(manager: &RepoManager, repo_id: RepoId) -> anyhow::Result<PreparedRepoIdentity> {
    let stem = repo_id.to_string();
    let local_dir = RepoManager::checked_local_dir_for(
        manager.ledger_dir(),
        "snapshotting prepared repo identity",
    )?;
    let db_path = local_dir.join(format!("{stem}.redb"));
    let repo_info = if manager.local_repo_name() == stem {
        manager
            .get_repo_info()?
            .ok_or_else(|| anyhow::anyhow!("repository metadata is missing for {repo_id}"))?
    } else {
        RepoManager::read_required_local_repo_info_from_path(
            &db_path,
            &stem,
            "snapshotting prepared repo identity",
        )?
    };
    if repo_info.uuid != repo_id {
        anyhow::bail!(
            "prepared DB identity mismatch: expected {repo_id}, got {}",
            repo_info.uuid
        );
    }

    let locator = manager
        .query_projection_locator_record_for_repo_id(repo_id)?
        .ok_or_else(|| anyhow::anyhow!("Projection Locator is missing for {repo_id}"))?;
    let workspace_segment = locator.workspace_segment.clone();
    let workspace_root =
        std::fs::canonicalize(locator.projection_base_abs.join(&workspace_segment))?;
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
