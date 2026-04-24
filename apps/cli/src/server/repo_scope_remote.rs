//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! 远端分支 selector 恢复辅助。
//!
//! Invariants:
//! - display-only 名称在无绑定 UUID 时不得升格为可执行 selector。
//! - UUID 形态输入若与真实 display name 冲突，必须 fail-closed。
//! - 返回 `Ok(Some(selector))` 时，结果必须可直接用于 remote repo 执行路径。

use super::repo_scope_error::stale_remote_scope_detail;
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
    let resolved = state.repo.find_remote_repo_selector(branch, repo_name)?;
    if resolved.as_deref() == Some(repo_name) {
        if let Some(expected_repo_id) = expected_repo_id
            && has_remote_display_name(state, branch, repo_name)?
            && let Some(selector) = state
                .repo
                .find_remote_repo_selector_by_id(branch, expected_repo_id)?
            && selector != repo_name
        {
            return Err(anyhow!(
                "{}",
                stale_remote_scope_detail(format!(
                    "Session repo mismatch: expected {}, resolved selector {} for exact repository selector {}",
                    expected_repo_id, selector, repo_name
                ))
            ));
        }
        return Ok(repo_name.to_string());
    }
    if uuid::Uuid::parse_str(repo_name).is_ok() {
        if has_remote_display_name(state, branch, repo_name)? {
            return Err(anyhow!(
                "{}",
                stale_remote_scope_detail(format!(
                    "Remote repository selector not resolved for {}",
                    repo_name
                ))
            ));
        }
        return Err(anyhow!(
            "{}",
            stale_remote_scope_detail(format!(
                "Remote repository selector not resolved for {}",
                repo_name
            ))
        ));
    }
    Err(anyhow!(
        "{}",
        stale_remote_scope_detail(format!(
            "Remote repository selector not resolved for {}",
            repo_name
        ))
    ))
}

fn has_remote_display_name(state: &Arc<AppState>, branch: &PeerId, raw_name: &str) -> Result<bool> {
    state.repo.has_remote_display_name(branch, raw_name)
}
