//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Switcher scope mutation and cleanup.

use crate::server::AppState;
use crate::server::repo_scope::{
    RepoScopeFailure, ResolvedRepo, map_repo_scope_error, map_repo_scope_error_ref,
    resolve_session_repo, stale_unbound_remote_scope_detail,
};
use crate::server::session::WsSession;
use crate::server::shadow_scope;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

mod cleanup;

pub(super) struct CurrentBranchSwitchContext {
    pub(super) scope: Option<ResolvedRepo>,
    pub(super) repo_url: Option<String>,
}

pub(super) fn clear_failed_current_scope(session: &mut WsSession, error: &ServerError) {
    cleanup::clear_failed_current_scope(session, error);
}

pub(super) fn resolve_current_branch_switch_context(
    state: &Arc<AppState>,
    session: &WsSession,
) -> Result<CurrentBranchSwitchContext, ServerError> {
    let scope = match resolve_session_repo(state, session) {
        Ok(scope) => Some(scope),
        Err(err) => {
            let mapped = map_current_scope_error(session, err);
            if can_ignore_missing_current_scope(session, mapped.code) {
                if let Some(branch) = session.active_branch.as_ref() {
                    ensure_current_remote_branch_for_switcher(state, branch)?;
                }
                None
            } else {
                return Err(mapped);
            }
        }
    };
    let repo_url = match scope.as_ref() {
        Some(scope) => match state
            .repo
            .get_repo_url(scope.branch.as_ref(), &scope.repo_name)
        {
            Ok(Some(url)) => Some(url),
            Ok(None) => {
                let kind = if scope.branch.is_some() {
                    "remote"
                } else {
                    "local"
                };
                return Err(map_repo_scope_error(anyhow::anyhow!(
                    "Broken {kind} repo {} while resolving current repo URL before branch switch: repository URL missing",
                    scope.repo_name
                )));
            }
            Err(err) => {
                return Err(map_repo_scope_error(anyhow::anyhow!(
                    "Failed to resolve current repo URL before branch switch: {}",
                    err
                )));
            }
        },
        None => recover_local_repo_url_from_hint(state, session)?,
    };
    Ok(CurrentBranchSwitchContext { scope, repo_url })
}

fn can_ignore_missing_current_scope(session: &WsSession, code: ServerErrorCode) -> bool {
    if session.active_repo.is_some()
        || session.active_repo_id.is_some()
        || session.has_runtime_scope_binding()
    {
        return false;
    }
    if code == ServerErrorCode::ScRepoContextInvalid {
        return session.active_branch.is_none();
    }
    matches!(
        code,
        ServerErrorCode::StorageNotFound | ServerErrorCode::SyncRepoUnbound
    )
}

fn map_current_scope_error(session: &WsSession, err: anyhow::Error) -> ServerError {
    if let Some(error) = RepoScopeFailure::from_anyhow(&err)
        && (error.is_remote_branch_unavailable()
            || matches!(error, RepoScopeFailure::StaleScope { .. }))
    {
        return ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, error.detail());
    }
    if let Some(branch) = session.active_branch.as_ref()
        && session.active_repo.is_none()
        && session.active_repo_id.is_none()
        && session.has_runtime_scope_binding()
    {
        let mapped = map_repo_scope_error_ref(&err);
        if mapped.code == ServerErrorCode::SyncRepoUnbound {
            return ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                stale_unbound_remote_scope_detail(branch),
            );
        }
    }
    let mapped = map_repo_scope_error_ref(&err);
    if session.active_branch.is_some()
        && mapped.code == ServerErrorCode::ScStaleScope
        && let Some(detail) = mapped.detail.as_deref()
        && detail.starts_with("stale remote scope:")
    {
        return ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, detail);
    }
    mapped
}

fn ensure_current_remote_branch_for_switcher(
    state: &Arc<AppState>,
    branch: &PeerId,
) -> Result<(), ServerError> {
    match shadow_scope::ensure_remote_branch_available(state, branch) {
        Ok(()) => Ok(()),
        Err(error) if error.is_remote_branch_unavailable() => Err(ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            error.detail(),
        )),
        Err(error) => Err(ServerError::from(error)),
    }
}

fn recover_local_repo_url_from_hint(
    _state: &Arc<AppState>,
    session: &WsSession,
) -> Result<Option<String>, ServerError> {
    if session.active_branch.is_some() {
        return Ok(None);
    }
    // can_ignore_missing_current_scope 已经阻止 active_repo.is_some() 的路径到达这里。
    debug_assert!(
        session.active_repo.is_none(),
        "recover_local_repo_url_from_hint reached with active_repo set"
    );
    Ok(None)
}
