//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime

use crate::server::session::WsSession;

pub(super) fn clear_remote_unbound_state(session: &mut WsSession) {
    session.clear_active_db();
    session.clear_sync_binding();
}

pub(super) fn clear_stale_non_browser_sync_scope(session: &mut WsSession) {
    if session.is_browser_session() {
        return;
    }
    clear_remote_unbound_state(session);
}

pub(super) fn clear_stale_browser_sync_scope(session: &mut WsSession) {
    if !session.is_browser_session() {
        return;
    }
    session.clear_active_db();
    session.clear_sync_binding();
}

pub(super) fn clear_invalid_sync_hello_scope(session: &mut WsSession) {
    if session.is_browser_session() {
        clear_stale_browser_sync_scope(session);
        return;
    }
    clear_stale_non_browser_sync_scope(session);
    session.clear_active_repo();
}

pub(super) fn clear_sync_hello_scope_failure(session: &mut WsSession, clear_active_repo: bool) {
    if clear_active_repo {
        clear_invalid_sync_hello_scope(session);
        return;
    }
    if session.is_browser_session() {
        clear_stale_browser_sync_scope(session);
        return;
    }
    clear_stale_non_browser_sync_scope(session);
}
