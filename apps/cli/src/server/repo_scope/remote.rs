//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! 远端分支 selector 恢复辅助。
//!
//! Invariants:
//! - display-only 名称在无绑定 UUID 时不得升格为可执行 selector。
//! - UUID 形态输入若与真实 display name 冲突，必须 fail-closed。
//! - 返回 `Ok(Some(selector))` 时，结果必须可直接用于 remote repo 执行路径。

use super::error::{RepoScopeFailure, stale_remote_scope_detail};
use crate::server::AppState;
use anyhow::{Result, anyhow};
use deve_core::models::{PeerId, RepoId};
use std::sync::Arc;

pub(super) fn recover_remote_repo_name_from_selector(
    state: &Arc<AppState>,
    branch: &PeerId,
    repo_name: &str,
    expected_repo_id: Option<RepoId>,
) -> Result<String> {
    if let Some(expected_repo_id) = expected_repo_id {
        let Some(expected_selector) = state
            .repo
            .find_remote_repo_selector_by_id(branch, expected_repo_id)?
        else {
            return Err(remote_selector_not_resolved(repo_name));
        };
        if uuid::Uuid::parse_str(repo_name).is_ok() {
            let resolved = state.repo.find_remote_repo_selector(branch, repo_name)?;
            if resolved.as_deref() == Some(expected_selector.as_str())
                && repo_name == expected_selector
            {
                return Ok(expected_selector);
            }
            if let Some(resolved) = resolved {
                return Err(RepoScopeFailure::exact_selector_mismatch(
                    stale_remote_scope_detail(format!(
                        "Session repo mismatch: expected {}, resolved selector {} for exact repository selector {}",
                        expected_repo_id, resolved, repo_name
                    )),
                )
                .into());
            }
            return Err(remote_selector_not_resolved(repo_name));
        }
        let Some(info) = state
            .repo
            .get_repo_info_for(Some(branch), Some(&expected_selector))?
        else {
            return Err(remote_selector_not_resolved(repo_name));
        };
        if info.uuid == expected_repo_id && info.name == repo_name {
            return Ok(expected_selector);
        }
        return Err(remote_selector_not_resolved(repo_name));
    }

    Err(remote_selector_not_resolved(repo_name))
}

fn remote_selector_not_resolved(repo_name: &str) -> anyhow::Error {
    anyhow!(
        "{}",
        stale_remote_scope_detail(format!(
            "Remote repository selector not resolved for {}",
            repo_name
        ))
    )
}
