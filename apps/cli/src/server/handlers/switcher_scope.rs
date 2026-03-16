use crate::server::AppState;
use crate::server::repo_scope::{ResolvedRepo, map_repo_scope_error, resolve_session_repo};
use crate::server::session::WsSession;
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
                None
            } else {
                return Err(mapped);
            }
        }
    };
    let repo_url = match scope.as_ref() {
        Some(scope) => state
            .repo
            .get_repo_url(scope.branch.as_ref(), &scope.repo_name)
            .map_err(|err| {
                map_repo_scope_error(anyhow::anyhow!(
                    "Failed to resolve current repo URL before branch switch: {}",
                    err
                ))
            })?,
        None => recover_local_repo_url_from_hint(state, session)?,
    };
    Ok(CurrentBranchSwitchContext { scope, repo_url })
}

fn can_ignore_missing_current_scope(session: &WsSession, code: ServerErrorCode) -> bool {
    if session.active_repo_id.is_some() {
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
    state: &Arc<AppState>,
    session: &WsSession,
) -> Result<Option<String>, ServerError> {
    if session.active_branch.is_some() {
        return Ok(None);
    }
    let Some(raw_name) = session.active_repo.as_deref() else {
        return Ok(None);
    };
    let Ok(repo_id) = uuid::Uuid::parse_str(raw_name) else {
        return Ok(None);
    };
    let Some(repo_name) = state
        .repo
        .find_local_repo_name_by_id(repo_id)
        .map_err(|err| map_repo_scope_error(anyhow::anyhow!(err.to_string())))?
    else {
        return Ok(None);
    };
    state
        .repo
        .get_repo_url(None, &repo_name)
        .map_err(|err| map_repo_scope_error(anyhow::anyhow!(err.to_string())))
}
