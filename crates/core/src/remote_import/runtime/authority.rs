//! plan_ref:
//!   - 03_storage/authority#remote-import-workflow-tables
//!   - 04_repository#repo-selector-resolution-contract

use crate::ledger::RepoManager;
use crate::models::RepoId;
use crate::remote_import::error::{RemoteImportError, RemoteImportResult};
use redb::Database;
use std::sync::Arc;

pub(super) fn bound_local_authority_db(
    repo: &RepoManager,
    repo_id: RepoId,
) -> RemoteImportResult<Arc<Database>> {
    if repo
        .is_local_repo_removed(repo_id)
        .map_err(RemoteImportError::storage)?
    {
        return Err(RemoteImportError::Storage(format!(
            "Remote Import local authority repo {repo_id} is removed"
        )));
    }
    let stem = repo_id.to_string();
    let db = if repo.local_repo_name == stem {
        repo.local_db.clone()
    } else {
        let local_dir = RepoManager::checked_local_dir_for(
            repo.ledger_dir(),
            "binding Remote Import local authority",
        )
        .map_err(RemoteImportError::storage)?;
        crate::ledger::database::cached_database(&local_dir.join(format!("{stem}.redb")))
            .map_err(RemoteImportError::storage)?
    };
    let info = RepoManager::read_local_repo_info_from_db(db.as_ref())
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
    Ok(db)
}
