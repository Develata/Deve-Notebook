use super::confirmed;
use crate::server::repo_scope::resolve_session_repo_and_sync;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::DocId;
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use std::sync::Arc;

pub(super) async fn handle_request_history(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    doc_id: DocId,
    request_id: u64,
) {
    let ops = match load_doc_history(state, session, doc_id) {
        Ok(ops) => ops,
        Err(err) => {
            ch.send_protocol_error(ServerError::with_detail(
                ServerErrorCode::RequestFailed,
                format!("Failed to load document history: {}", err),
            ));
            return;
        }
    };
    ch.unicast(ServerMessage::History {
        doc_id,
        request_id,
        ops,
    });
}

fn load_doc_history(
    state: &Arc<AppState>,
    session: &mut WsSession,
    doc_id: DocId,
) -> anyhow::Result<Vec<deve_core::protocol::ConfirmedOp>> {
    if let Some(handle) = session.get_active_db() {
        return confirmed::load_doc_ops(&handle.db, doc_id);
    }
    let scope = resolve_session_repo_and_sync(state, session)?;
    state.repo.run_on_local_repo(&scope.repo_name, |db| {
        confirmed::load_doc_ops(db, doc_id)
    })
}
