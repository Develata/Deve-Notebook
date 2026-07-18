//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! 会话级 repo selector 解析。
//!
//! Invariants:
//! - 本地 `active_repo_id` 存在时必须先按 RepoId 解析 canonical execution stem，再复核
//!   `active_repo` 等于该 stem 或 metadata display alias；stale pair 必须 fail-closed。
//! - 无 `active_repo_id` 的 UUID-shaped local selector 必须 fail-closed。
//! - 远端分支只有在 selector 可由当前 branch 唯一恢复时才允许继续执行。
//! - 本模块只负责把 session hint 收敛成可执行 selector；不负责最终 `RepoUUID` 解析。

use super::error::stale_remote_scope_detail;
use super::remote::recover_remote_repo_name_from_selector;
use crate::server::AppState;
use crate::server::error_classify::{is_db_locked, is_storage_corruption};
use crate::server::session::WsSession;
use crate::server::shadow_scope;
use anyhow::{Result, anyhow};
use std::sync::Arc;

pub(super) fn resolve_repo_name_from_session(
    state: &Arc<AppState>,
    session: &WsSession,
) -> Result<Option<String>> {
    if session.active_branch.is_none() {
        return resolve_local_repo_name_from_session(state, session);
    }
    resolve_remote_repo_name_from_session(state, session)
}

fn resolve_local_repo_name_from_session(
    state: &Arc<AppState>,
    session: &WsSession,
) -> Result<Option<String>> {
    if let Some(repo_name) = session.active_repo.clone() {
        if let Some(bound_id) = session.active_repo_id {
            let Some(resolved) = state.repo.find_local_repo_name_by_id(bound_id)? else {
                return Err(local_selector_not_resolved(&repo_name));
            };
            let Some(info) = state.repo.get_repo_info_for(None, Some(&resolved))? else {
                return Err(local_selector_not_resolved(&repo_name));
            };
            if repo_name != resolved && repo_name != info.name {
                return Err(local_selector_not_resolved(&repo_name));
            }
            return Ok(Some(resolved));
        }
        if uuid::Uuid::parse_str(&repo_name).is_ok() {
            return Err(local_selector_not_resolved(&repo_name));
        }
        let resolved = match state
            .repo
            .resolve_local_repo_name_for_execution(None, Some(&repo_name))
        {
            Ok(resolved) => resolved,
            Err(err) if should_preserve_local_selector_error(&err) => {
                return Err(err);
            }
            Err(_) => {
                return Err(local_selector_not_resolved(&repo_name));
            }
        };
        return Ok(Some(resolved));
    }
    if let Some(repo_id) = session.active_repo_id {
        return Err(anyhow!(
            "Local repository selector not resolved for {}",
            repo_id
        ));
    }
    Ok(None)
}

fn local_selector_not_resolved(selector: &str) -> anyhow::Error {
    anyhow!("Local repository selector not resolved for {}", selector)
}

fn resolve_remote_repo_name_from_session(
    state: &Arc<AppState>,
    session: &WsSession,
) -> Result<Option<String>> {
    if let Some(branch) = session.active_branch.as_ref() {
        shadow_scope::ensure_remote_branch_available(state, branch)?;
    }
    if let Some(repo_name) = session.active_repo.clone() {
        let Some(branch) = session.active_branch.as_ref() else {
            return Ok(Some(repo_name));
        };
        let selector = recover_remote_repo_name_from_selector(
            state,
            branch,
            &repo_name,
            session.active_repo_id,
        )?;
        return Ok(Some(selector));
    }
    let Some(repo_id) = session.active_repo_id else {
        return Ok(None);
    };
    let Some(branch) = session.active_branch.as_ref() else {
        return Err(anyhow!(
            "Local repository selector not resolved for {}",
            repo_id
        ));
    };
    if let Some(selector) = state
        .repo
        .find_remote_repo_selector_by_id(branch, repo_id)?
    {
        return Ok(Some(selector));
    }
    Err(anyhow!(
        "{}",
        stale_remote_scope_detail(format!(
            "Remote session lost repo name for bound repo {}",
            repo_id
        ))
    ))
}

fn should_preserve_local_selector_error(err: &anyhow::Error) -> bool {
    let lower = err.to_string().to_ascii_lowercase();
    is_storage_corruption(&lower) || is_db_locked(&lower)
}
