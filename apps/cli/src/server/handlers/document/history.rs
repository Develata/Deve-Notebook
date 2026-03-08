use crate::server::repo_scope::resolve_session_repo;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::DocId;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

pub(super) async fn handle_request_history(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    doc_id: DocId,
    request_id: u64,
) {
    let Ok(scope) = resolve_session_repo(state, session) else {
        return;
    };
    if let Ok(entries) = state
        .repo
        .get_local_ops_in_local_repo(&scope.repo_name, doc_id)
    {
        let ops = entries
            .into_iter()
            .map(|(seq, entry)| (seq, entry.op))
            .collect();
        ch.unicast(ServerMessage::History {
            doc_id,
            request_id,
            ops,
        });
    }
}
