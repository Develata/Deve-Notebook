//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Scope-guarded core WebSocket message routing.

use crate::server::handlers::{document, key_exchange, listing, search, sync};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::ClientMessage;
use std::sync::Arc;

pub(super) async fn route_scoped_core(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    msg: ClientMessage,
) -> Option<ClientMessage> {
    let Some(scope) = msg.core_scope_gate() else {
        return Some(msg);
    };
    if super::scope_guard::reject_invalid_browser_scope_nonce(
        ch,
        session,
        scope.scope_nonce,
        scope.scope_name,
    ) {
        return None;
    }
    match msg {
        ClientMessage::OpenDoc {
            doc_id, request_id, ..
        } => document::handle_open_doc(state, ch, session, doc_id, request_id).await,
        ClientMessage::RequestHistory {
            doc_id, request_id, ..
        } => document::handle_request_history(state, ch, session, doc_id, request_id).await,
        ClientMessage::Edit {
            doc_id,
            op,
            client_id,
            client_op_id,
            scope_nonce,
        } => {
            document::handle_edit(
                state,
                ch,
                session,
                document::EditRequest {
                    doc_id,
                    op,
                    client_id,
                    client_op_id,
                    scope_nonce,
                },
            )
            .await
        }
        ClientMessage::ListDocs { request_id, .. } => {
            listing::handle_list_docs(state, ch, session, Some(request_id), None).await
        }
        ClientMessage::ListShadows { request_id, .. } => {
            listing::handle_list_shadows(state, ch, Some(session), Some(request_id)).await
        }
        ClientMessage::ListRepos { request_id, .. } => {
            listing::handle_list_repos(state, ch, session, Some(request_id)).await
        }
        ClientMessage::Search {
            request_id,
            query,
            limit,
            scope_nonce,
        } => search::handle_search(state, ch, session, request_id, query, limit, scope_nonce).await,
        ClientMessage::DeletePeer { peer_id, .. } => {
            sync::handle_delete_peer(state, ch, session, peer_id).await
        }
        ClientMessage::RequestKey { .. } => {
            key_exchange::handle_request_key(state, ch, session).await
        }
        other => return Some(other),
    }
    None
}

#[cfg(test)]
mod tests;
