//! 远端分支 selector 恢复辅助。
//!
//! Invariants:
//! - display-only 名称在无绑定 UUID 时不得升格为可执行 selector。
//! - UUID 形态输入若与真实 display name 冲突，必须 fail-closed。
//! - 返回 `Ok(Some(selector))` 时，结果必须可直接用于 remote repo 执行路径。

use crate::server::AppState;
use anyhow::{Result, anyhow};
use deve_core::models::{PeerId, RepoId};
use std::sync::Arc;

pub(super) fn recover_remote_repo_name_from_selector(
    state: &Arc<AppState>,
    branch: &PeerId,
    repo_name: &str,
    expected_repo_id: Option<RepoId>,
) -> Result<Option<String>> {
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
                "Session repo mismatch: expected {}, resolved selector {} for exact repository selector {}",
                expected_repo_id,
                selector,
                repo_name
            ));
        }
        return Ok(resolved);
    }
    if uuid::Uuid::parse_str(repo_name).is_ok() {
        if has_remote_display_name(state, branch, repo_name)? {
            tracing::warn!(
                "Refusing to recover UUID-shaped remote selector without bound UUID because matching display name exists: branch={}, raw_name={}, resolved_selector={:?}",
                branch,
                repo_name,
                resolved
            );
            return Ok(None);
        }
        tracing::warn!(
            "Refusing to recover UUID-shaped remote selector from repo_name slot during runtime scope resolution: branch={}, raw_name={}, expected_repo_id={:?}",
            branch,
            repo_name,
            expected_repo_id
        );
        return Ok(None);
    }
    tracing::warn!(
        "Refusing to recover remote repo selector from display-only name without UUID: branch={}, raw_name={}, resolved_selector={:?}",
        branch,
        repo_name,
        resolved
    );
    Ok(None)
}

fn has_remote_display_name(state: &Arc<AppState>, branch: &PeerId, raw_name: &str) -> Result<bool> {
    state.repo.has_remote_display_name(branch, raw_name)
}
