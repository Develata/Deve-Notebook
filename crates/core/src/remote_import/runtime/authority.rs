//! plan_ref:
//!   - 03_storage/authority#remote-import-workflow-tables
//!   - 04_repository#repo-selector-resolution-contract

use crate::ledger::RepoManager;
use crate::ledger::manager::BoundRepoAuthority;
use crate::models::RepoId;
use crate::remote_import::error::{RemoteImportError, RemoteImportResult};

pub(super) fn bound_local_authority_db(
    repo: &RepoManager,
    repo_id: RepoId,
) -> RemoteImportResult<BoundRepoAuthority> {
    if repo
        .is_local_repo_removed(repo_id)
        .map_err(RemoteImportError::storage)?
    {
        return Err(RemoteImportError::Storage(format!(
            "Remote Import local authority repo {repo_id} is removed"
        )));
    }
    let authority = repo
        .bind_local_authority(repo_id)
        .map_err(RemoteImportError::storage)?;
    let lease = authority.lease().map_err(RemoteImportError::storage)?;
    let info = RepoManager::read_local_repo_info_from_db(lease.db())
        .map_err(RemoteImportError::storage)?
        .ok_or_else(|| {
            RemoteImportError::Storage(
                "Remote Import local authority RepoInfo is missing".to_string(),
            )
        })?;
    if info.uuid != repo_id {
        return Err(RemoteImportError::Storage(format!(
            "Remote Import physical RepoId {repo_id} does not match local authority RepoId {}",
            info.uuid
        )));
    }
    drop(lease);
    Ok(authority)
}
