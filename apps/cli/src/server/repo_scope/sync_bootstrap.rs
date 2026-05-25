//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Local bootstrap retry path for synchronized repo scope resolution.

use super::{ResolvedRepo, bootstrap_local_repo, resolve_session_repo_and_sync};
use crate::server::{AppState, session::WsSession};
use anyhow::Result;
use std::sync::Arc;

/// 在本地 single-repo 入口上，允许先清 stale runtime binding，再重新执行 local bootstrap。
///
/// Invariants:
/// - 仅当当前会话没有 `active_branch/active_repo/active_repo_id` 时才允许 bootstrap。
/// - 若 stale local binding 已被 `resolve_session_repo_and_sync` 清理干净，则允许重试 bootstrap。
pub fn resolve_session_repo_or_bootstrap_local(
    state: &Arc<AppState>,
    session: &mut WsSession,
) -> Result<ResolvedRepo> {
    if !wants_local_bootstrap(session) {
        return resolve_session_repo_and_sync(state, session);
    }
    if !session.has_runtime_scope_binding() {
        return bootstrap_and_bind_local_repo(state, session);
    }
    match resolve_session_repo_and_sync(state, session) {
        Ok(scope) => Ok(scope),
        Err(_) if wants_local_bootstrap(session) && !session.has_runtime_scope_binding() => {
            bootstrap_and_bind_local_repo(state, session)
        }
        Err(err) => Err(err),
    }
}

fn bootstrap_and_bind_local_repo(
    state: &Arc<AppState>,
    session: &mut WsSession,
) -> Result<ResolvedRepo> {
    let scope = bootstrap_local_repo(state, session)?;
    session.switch_repo(scope.repo_name.clone(), Some(scope.repo_id));
    Ok(scope)
}

fn wants_local_bootstrap(session: &WsSession) -> bool {
    session.active_branch.is_none()
        && session.active_repo.is_none()
        && session.active_repo_id.is_none()
}
