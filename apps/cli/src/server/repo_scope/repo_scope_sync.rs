//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Session repo scope binding synchronization.

use super::repo_scope_cleanup::{should_clear_stale_local_scope, should_clear_stale_remote_scope};
use super::{
    RepoScopeFailure, ResolvedRepo, map_repo_scope_error_ref, resolve_session_repo,
    stale_unbound_remote_scope_detail,
};
use crate::server::{AppState, session::WsSession, shadow_scope};
use anyhow::Result;
use deve_core::protocol::ServerErrorCode;
use std::sync::Arc;

/// 解析并回写会话中的 repo 绑定，收敛 stale `active_repo_id/name`。
/// Invariants: 会话级 repo-scoped 读写应尽量先调用本函数；若解析结果与会话不一致，以解析结果为准。
pub fn resolve_session_repo_and_sync(
    state: &Arc<AppState>,
    session: &mut WsSession,
) -> Result<ResolvedRepo> {
    let scope = match resolve_session_repo(state, session) {
        Ok(scope) => scope,
        Err(err) => return handle_resolution_error(session, err),
    };
    sync_runtime_binding(session, &scope);
    Ok(scope)
}

fn handle_resolution_error(session: &mut WsSession, err: anyhow::Error) -> Result<ResolvedRepo> {
    let err = normalize_unbound_remote_scope_error(session, err);
    let mapped = map_repo_scope_error_ref(&err);
    if session.active_branch.is_some()
        && RepoScopeFailure::from_anyhow(&err)
            .is_some_and(RepoScopeFailure::is_remote_branch_unavailable)
    {
        shadow_scope::clear_stale_remote_branch(session);
        return Err(err);
    }
    if should_clear_stale_scope(session, &mapped) {
        session.clear_active_repo();
        session.clear_active_db();
        session.clear_sync_binding();
    }
    Err(err)
}

fn normalize_unbound_remote_scope_error(session: &WsSession, err: anyhow::Error) -> anyhow::Error {
    if session.active_branch.is_some()
        && session.active_repo.is_none()
        && session.active_repo_id.is_none()
        && session.has_runtime_scope_binding()
        && map_repo_scope_error_ref(&err).code == ServerErrorCode::SyncRepoUnbound
    {
        return RepoScopeFailure::stale_scope(stale_unbound_remote_scope_detail(
            session
                .active_branch
                .as_ref()
                .expect("checked active branch"),
        ))
        .into();
    }
    err
}

fn should_clear_stale_scope(
    session: &WsSession,
    mapped: &deve_core::protocol::ServerError,
) -> bool {
    if session.active_branch.is_some() {
        return should_clear_stale_remote_scope(mapped);
    }
    should_clear_stale_local_scope(mapped)
        || (mapped.code == ServerErrorCode::SyncRepoUnbound && session.has_runtime_scope_binding())
}

fn sync_runtime_binding(session: &mut WsSession, scope: &ResolvedRepo) {
    if runtime_binding_mismatch(session, scope) {
        session.clear_active_db();
        session.clear_sync_binding();
    }
    if session.active_repo.as_deref() != Some(scope.repo_name.as_str())
        || session.active_repo_id != Some(scope.repo_id)
    {
        session.switch_repo(scope.repo_name.clone(), Some(scope.repo_id));
    }
}

fn runtime_binding_mismatch(session: &WsSession, scope: &ResolvedRepo) -> bool {
    let active_db_mismatch = session.get_active_db().is_some()
        && session
            .active_db_for(scope.branch.as_ref(), &scope.repo_name, Some(scope.repo_id))
            .is_none();
    let bound_repo_mismatch = session
        .bound_repo_id
        .is_some_and(|repo_id| repo_id != scope.repo_id);
    let writer_mismatch = session
        .writer_identity
        .as_ref()
        .is_some_and(|writer| writer.repo_id != scope.repo_id);
    active_db_mismatch || bound_repo_mismatch || writer_mismatch
}
