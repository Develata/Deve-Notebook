use super::ResolvedRepo;
use anyhow::{Result, anyhow};
use crate::server::AppState;
use deve_core::models::{PeerId, RepoId};
use std::sync::Arc;

pub(super) fn resolve_repo_by_name(
    state: &Arc<AppState>,
    branch: Option<PeerId>,
    expected_repo_id: Option<RepoId>,
    repo_name: String,
) -> Result<ResolvedRepo> {
    let info = state
        .repo
        .get_repo_info_for(branch.as_ref(), Some(&repo_name))?
        .ok_or_else(|| anyhow!("Repository UUID not resolved for {}", repo_name))?;
    if let Some(expected_repo_id) = expected_repo_id
        && expected_repo_id != info.uuid
    {
        return Err(anyhow!(
            "Session repo mismatch: expected {}, resolved {} for {}",
            expected_repo_id,
            info.uuid,
            repo_name
        ));
    }
    Ok(ResolvedRepo {
        repo_id: info.uuid,
        repo_name,
        branch,
    })
}

pub(super) fn resolve_repo_by_repo_id(
    state: &Arc<AppState>,
    branch: Option<PeerId>,
    repo_id: RepoId,
) -> Result<ResolvedRepo> {
    let info = state
        .repo
        .get_repo_info_for(branch.as_ref(), Some(&repo_id.to_string()))?
        .ok_or_else(|| anyhow!("Repository UUID not resolved for {}", repo_id))?;
    Ok(ResolvedRepo {
        repo_id,
        repo_name: info.name,
        branch,
    })
}
