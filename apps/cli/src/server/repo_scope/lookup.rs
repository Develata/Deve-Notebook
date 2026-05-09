//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! `repo_name` / `repo_id` 收敛到单一 `ResolvedRepo`。
//!
//! Invariants:
//! - 执行前必须把本地/远端输入收敛到唯一 `RepoUUID`。
//! - 对远端分支，exact selector 优先于 stale UUID；冲突时必须 fail-closed。
//! - 对本地分支，不允许借 stale `repo_id` 反向覆盖当前 selector。

use super::ResolvedRepo;
use super::error::{RepoScopeFailure, stale_remote_scope_detail};
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
            return Err(anyhow!(
                "{}",
                missing_repo_uuid_detail(branch.as_ref(), &repo_name)
            ));
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
            return Err(
                RepoScopeFailure::exact_selector_mismatch(repo_mismatch_detail(
                    branch.as_ref(),
                    expected_repo_id,
                    info.uuid,
                    &repo_name,
                    true,
                ))
                .into(),
            );
        }
        return Err(anyhow!(
            "{}",
            repo_mismatch_detail(
                branch.as_ref(),
                expected_repo_id,
                info.uuid,
                &repo_name,
                false
            )
        ));
    }
    let repo_name = match recover_repo_selector(state, branch.as_ref(), info.uuid)? {
        Some(selector) => selector,
        None if branch.is_some() => {
            return Err(anyhow!(
                "{}",
                stale_remote_scope_detail(format!(
                    "Remote repository selector not resolved for {}",
                    repo_name
                ))
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

fn missing_repo_uuid_detail(branch: Option<&PeerId>, repo_name: &str) -> String {
    match branch {
        Some(_) => {
            stale_remote_scope_detail(format!("Repository UUID not resolved for {}", repo_name))
        }
        None => format!("Repository UUID not resolved for {}", repo_name),
    }
}

fn repo_mismatch_detail(
    branch: Option<&PeerId>,
    expected_repo_id: RepoId,
    resolved_repo_id: RepoId,
    repo_name: &str,
    exact_selector: bool,
) -> String {
    let detail = if exact_selector {
        format!(
            "Session repo mismatch: expected {}, resolved {} for exact repository selector {}",
            expected_repo_id, resolved_repo_id, repo_name
        )
    } else {
        format!(
            "Session repo mismatch: expected {}, resolved {} for {}",
            expected_repo_id, resolved_repo_id, repo_name
        )
    };
    match branch {
        Some(_) => stale_remote_scope_detail(detail),
        None => detail,
    }
}
