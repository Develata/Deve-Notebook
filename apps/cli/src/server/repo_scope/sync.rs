//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Session repo scope binding synchronization.

use super::cleanup::{should_clear_stale_local_scope, should_clear_stale_remote_scope};
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
        Err(err) => return handle_resolution_error(state, session, err),
    };
    sync_runtime_binding(state, session, &scope)?;
    Ok(scope)
}

fn handle_resolution_error(
    state: &Arc<AppState>,
    session: &mut WsSession,
    err: anyhow::Error,
) -> Result<ResolvedRepo> {
    let err = normalize_unbound_remote_scope_error(session, err);
    let mapped = map_repo_scope_error_ref(&err);
    if session.active_branch.is_some()
        && RepoScopeFailure::from_anyhow(&err)
            .is_some_and(RepoScopeFailure::is_remote_branch_unavailable)
    {
        state.revoke_source_control_write_grant_for_session(session);
        shadow_scope::clear_stale_remote_branch(session);
        return Err(err);
    }
    if should_clear_stale_scope(session, &mapped) {
        clear_runtime_binding_and_revoke(state, session);
        session.clear_active_repo();
    }
    Err(err)
}

fn normalize_unbound_remote_scope_error(session: &WsSession, err: anyhow::Error) -> anyhow::Error {
    if let Some(branch) = session.active_branch.as_ref()
        && session.active_repo.is_none()
        && session.active_repo_id.is_none()
        && session.has_runtime_scope_binding()
        && map_repo_scope_error_ref(&err).code == ServerErrorCode::SyncRepoUnbound
    {
        return RepoScopeFailure::stale_scope(stale_unbound_remote_scope_detail(branch)).into();
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

fn sync_runtime_binding(
    state: &Arc<AppState>,
    session: &mut WsSession,
    scope: &ResolvedRepo,
) -> Result<()> {
    if runtime_binding_mismatch(session, scope) {
        clear_runtime_binding_and_revoke(state, session);
    }
    if session.active_repo.as_deref() != Some(scope.session_name.as_str())
        || session.active_repo_id != Some(scope.repo_id)
    {
        session.switch_repo(scope.session_name.clone(), Some(scope.repo_id));
    }
    if scope.branch.is_none() {
        let membership_valid = session
            .catalog_membership_token()
            .filter(|token| token.repo_id() == scope.repo_id)
            .is_some_and(|token| state.catalog_membership_runtime().revalidate(token).is_ok());
        if !membership_valid {
            let token = state
                .catalog_membership_runtime()
                .issue(scope.repo_id)
                .map_err(|error| {
                    tracing::warn!(repo_id = %scope.repo_id, %error, "repo catalog membership unavailable during scope synchronization");
                    anyhow::Error::new(error)
                })?;
            session.bind_catalog_membership(token);
        }
    }
    Ok(())
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

fn clear_runtime_binding_and_revoke(state: &Arc<AppState>, session: &mut WsSession) {
    state.revoke_source_control_write_grant_for_session(session);
    session.clear_active_db();
    session.clear_sync_binding();
}
