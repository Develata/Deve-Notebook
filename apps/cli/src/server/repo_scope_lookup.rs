//! `repo_name` / `repo_id` 收敛到单一 `ResolvedRepo`。
//!
//! Invariants:
//! - 执行前必须把本地/远端输入收敛到唯一 `RepoUUID`。
//! - 对远端分支，exact selector 优先于 stale UUID；冲突时必须 fail-closed。
//! - 对本地分支，不允许借 stale `repo_id` 反向覆盖当前 selector。

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
    let repo_name = match recover_repo_selector(state, branch.as_ref(), info.uuid)? {
        Some(selector) => selector,
        None if branch.is_some() => {
            return Err(anyhow!(
                "Remote repository selector not resolved for {}",
                repo_name
            ));
        }
        None => repo_name,
    };
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
