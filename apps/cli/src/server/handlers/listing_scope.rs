use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{map_repo_scope_error, stale_unbound_remote_scope_detail};
use crate::server::session::WsSession;
use crate::server::shadow_scope;
use anyhow::anyhow;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

pub(super) fn clear_local_unbound_runtime_binding(session: &mut WsSession) {
    if session.active_branch.is_none()
        && session.active_repo.is_none()
        && session.active_repo_id.is_none()
        && session.has_runtime_scope_binding()
    {
        session.clear_active_db();
        session.clear_sync_binding();
    }
}

pub(super) fn precheck_remote_unbound_scope(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    scope_nonce: Option<u64>,
) -> bool {
    let Some(branch) = session.active_branch.as_ref().cloned() else {
        return false;
    };
    if session.active_repo.is_some() || session.active_repo_id.is_some() {
        return false;
    }
    if let Err(error) = shadow_scope::map_remote_branch_availability(state, &branch) {
        if shadow_scope::should_clear_missing_remote_branch(&error) {
            shadow_scope::clear_stale_remote_branch(session);
        } else {
            session.clear_active_db();
            session.clear_sync_binding();
        }
        ch.send_protocol_error_with_scope_nonce(error, scope_nonce);
        return true;
    }
    if session.has_runtime_scope_binding() {
        session.clear_active_db();
        session.clear_sync_binding();
        ch.send_protocol_error_with_scope_nonce(
            ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                stale_unbound_remote_scope_detail(&branch),
            ),
            scope_nonce,
        );
        return true;
    }
    false
}

pub(super) fn send_listing_error(
    ch: &DualChannel,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    let detail = detail.into();
    ch.send_protocol_error_with_scope_nonce(
        ServerError::with_detail(classify_listing_error(&detail), detail),
        scope_nonce,
    );
}

pub(super) fn browser_scope_nonce(session: Option<&WsSession>) -> Option<u64> {
    session
        .filter(|session| session.is_browser_session())
        .map(WsSession::scope_nonce)
}

fn classify_listing_error(detail: &str) -> ServerErrorCode {
    map_repo_scope_error(anyhow!(detail.to_string())).code
}

#[cfg(test)]
#[path = "listing_scope_test.rs"]
mod tests;
