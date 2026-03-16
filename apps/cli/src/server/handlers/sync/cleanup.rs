use crate::server::session::WsSession;

pub(super) fn clear_remote_unbound_state(session: &mut WsSession) {
    if session.active_branch.is_some() {
        session.clear_active_db();
        session.clear_sync_binding();
    }
}

pub(super) fn clear_stale_browser_sync_scope(session: &mut WsSession) {
    if !session.is_browser_session() {
        return;
    }
    session.clear_active_db();
    session.clear_sync_binding();
}
