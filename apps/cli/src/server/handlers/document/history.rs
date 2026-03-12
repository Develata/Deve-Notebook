use super::confirmed;
use crate::server::repo_scope::resolve_session_repo;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::DocId;
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use std::sync::Arc;

pub(super) async fn handle_request_history(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    doc_id: DocId,
    request_id: u64,
) {
    let scope = match resolve_session_repo(state, session) {
        Ok(scope) => scope,
        Err(err) => {
            ch.send_protocol_error(ServerError::with_detail(
                ServerErrorCode::RequestFailed,
                err.to_string(),
            ));
            return;
        }
    };
    let ops = match state
        .repo
        .run_on_local_repo(&scope.repo_name, |db| confirmed::load_doc_ops(db, doc_id))
    {
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
