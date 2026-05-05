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
    let Some((scope_nonce, scope_name)) = requested_scope_nonce(&msg) else {
        return Some(msg);
    };
    if let Err(error) =
        super::scope_guard::validate_browser_scope_nonce(session, scope_nonce, scope_name)
    {
        ch.send_protocol_error_with_scope_nonce(
            error,
            super::scope_guard::response_scope_nonce(session, scope_nonce),
        );
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

fn requested_scope_nonce(msg: &ClientMessage) -> Option<(Option<u64>, &'static str)> {
    match msg {
        ClientMessage::OpenDoc { scope_nonce, .. } => Some((*scope_nonce, "open doc")),
        ClientMessage::RequestHistory { scope_nonce, .. } => {
            Some((*scope_nonce, "document history"))
        }
        ClientMessage::Edit { scope_nonce, .. } => Some((*scope_nonce, "edit")),
        ClientMessage::ListDocs { scope_nonce, .. } => Some((*scope_nonce, "document list")),
        ClientMessage::ListShadows { scope_nonce, .. } => Some((*scope_nonce, "shadow list")),
        ClientMessage::ListRepos { scope_nonce, .. } => Some((*scope_nonce, "repo list")),
        ClientMessage::Search { scope_nonce, .. } => Some((*scope_nonce, "search")),
        ClientMessage::DeletePeer { scope_nonce, .. } => Some((*scope_nonce, "delete peer")),
        ClientMessage::RequestKey { scope_nonce } => Some((*scope_nonce, "request key")),
        _ => None,
    }
}

#[cfg(test)]
#[path = "core_scoped_test.rs"]
mod tests;
