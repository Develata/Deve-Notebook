use super::ResolvedRepo;
use crate::server::AppState;
use anyhow::{Result, anyhow};
use deve_core::models::{PeerId, RepoId};
use std::sync::Arc;

pub(super) fn resolve_repo_by_name(
    state: &Arc<AppState>,
    branch: Option<PeerId>,
    expected_repo_id: Option<RepoId>,
    repo_name: String,
) -> Result<ResolvedRepo> {
    let info = match state
        .repo
        .get_repo_info_for(branch.as_ref(), Some(&repo_name))?
    {
        Some(info) => info,
        None => {
            if branch.is_some()
                && let Some(expected_repo_id) = expected_repo_id
                && let Some(selector) =
                    recover_repo_selector(state, branch.as_ref(), expected_repo_id)?
            {
                tracing::warn!(
                    "Recovering repo selector from UUID after stale name miss: branch={:?}, stale_name={}, resolved_name={}",
                    branch,
                    repo_name,
                    selector
                );
                return Ok(ResolvedRepo {
                    repo_id: expected_repo_id,
                    repo_name: selector,
                    branch,
                });
            }
            return Err(anyhow!("Repository UUID not resolved for {}", repo_name));
        }
    };
    if let Some(expected_repo_id) = expected_repo_id
        && expected_repo_id != info.uuid
    {
        if let Some(peer_id) = branch.as_ref()
            && state
                .repo
                .find_remote_repo_selector(peer_id, &repo_name)?
                .as_deref()
                == Some(repo_name.as_str())
        {
            return Err(anyhow!(
                "Session repo mismatch: expected {}, resolved {} for exact repository selector {}",
                expected_repo_id,
                info.uuid,
                repo_name
            ));
        }
        if branch.is_some()
            && let Some(selector) = recover_repo_selector(state, branch.as_ref(), expected_repo_id)?
        {
            tracing::warn!(
                "Recovering repo selector from UUID after mismatch: branch={:?}, stale_name={}, resolved_name={}",
                branch,
                repo_name,
                selector
            );
            return Ok(ResolvedRepo {
                repo_id: expected_repo_id,
                repo_name: selector,
                branch,
            });
        }
        return Err(anyhow!(
            "Session repo mismatch: expected {}, resolved {} for {}",
            expected_repo_id,
            info.uuid,
            repo_name
        ));
    }
    let repo_name = recover_repo_selector(state, branch.as_ref(), info.uuid)?.unwrap_or(repo_name);
    Ok(ResolvedRepo {
        repo_id: info.uuid,
        repo_name,
        branch,
    })
}

fn recover_repo_selector(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    repo_id: RepoId,
) -> Result<Option<String>> {
    if let Some(peer_id) = branch {
        return state.repo.find_remote_repo_selector_by_id(peer_id, repo_id);
    }
    state.repo.find_local_repo_name_by_id(repo_id)
}
