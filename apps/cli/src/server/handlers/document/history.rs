use crate::server::{AppState, channel::DualChannel};
use deve_core::models::DocId;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

pub(super) async fn handle_request_history(state: &Arc<AppState>, ch: &DualChannel, doc_id: DocId) {
    if let Ok(entries) = state.repo.get_local_ops(doc_id) {
        let ops = entries
            .into_iter()
            .map(|(seq, entry)| (seq, entry.op))
            .collect();
        ch.unicast(ServerMessage::History { doc_id, ops });
    }
}
