use crate::server::AppState;
use crate::server::repo_scope::{
    ResolvedRepo, map_repo_scope_error, resolve_session_repo, should_clear_stale_remote_scope,
};
use crate::server::session::WsSession;
use crate::server::shadow_scope;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

pub(super) struct CurrentBranchSwitchContext {
    pub(super) scope: Option<ResolvedRepo>,
    pub(super) repo_url: Option<String>,
}

pub(super) fn resolve_current_branch_switch_context(
    state: &Arc<AppState>,
    session: &WsSession,
) -> Result<CurrentBranchSwitchContext, ServerError> {
    let scope = match resolve_session_repo(state, session) {
        Ok(scope) => Some(scope),
        Err(err) => {
            let mapped = map_repo_scope_error(anyhow::anyhow!(err.to_string()));
            if can_ignore_missing_current_scope(session, mapped.code) {
                if let Some(branch) = session.active_branch.as_ref() {
                    shadow_scope::map_remote_branch_availability(state, branch)?;
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

pub(super) fn clear_failed_current_scope(session: &mut WsSession, error: &ServerError) {
    if !should_clear_failed_current_scope(session, error) {
        return;
    }
    if session.active_branch.is_some() && shadow_scope::should_clear_missing_remote_branch(error) {
        shadow_scope::clear_stale_remote_branch(session);
        return;
    }
    session.clear_active_repo();
    session.clear_active_db();
    session.clear_sync_binding();
}

fn can_ignore_missing_current_scope(session: &WsSession, code: ServerErrorCode) -> bool {
    if session.active_repo.is_some()
        || session.active_repo_id.is_some()
        || has_runtime_scope_binding(session)
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

fn recover_local_repo_url_from_hint(
    _state: &Arc<AppState>,
    session: &WsSession,
) -> Result<Option<String>, ServerError> {
    if session.active_branch.is_some() {
        return Ok(None);
    }
    let Some(raw_name) = session.active_repo.as_deref() else {
        return Ok(None);
    };
    if uuid::Uuid::parse_str(raw_name).is_ok() {
        return Err(ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            format!("Local repository selector not resolved for {}", raw_name),
        ));
    }
    Ok(None)
}

fn should_clear_failed_current_scope(session: &WsSession, error: &ServerError) -> bool {
    if session.active_branch.is_some() {
        return match error.code {
            ServerErrorCode::SyncRepoUnbound | ServerErrorCode::ScRepoContextInvalid => {
                should_clear_stale_remote_scope(error)
            }
            _ => false,
        };
    }
    matches!(
        error.code,
        ServerErrorCode::ScRepoContextInvalid | ServerErrorCode::StorageNotFound
    ) || (error.code == ServerErrorCode::SyncRepoUnbound && has_runtime_scope_binding(session))
}

fn has_runtime_scope_binding(session: &WsSession) -> bool {
    session.get_active_db().is_some()
        || session.authenticated_peer_id.is_some()
        || session.bound_repo_id.is_some()
        || session.sync_scope_nonce().is_some()
        || session.writer_identity.is_some()
}
