use crate::server::handlers::{document, key_exchange, listing, plugin, search, switcher, sync};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::ClientMessage;
use std::sync::Arc;

/// 路由剩余的核心消息（内容、查询、切换、快照同步等）。
pub(super) async fn route_core(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    msg: ClientMessage,
) {
    match msg {
        ClientMessage::OpenDoc {
            doc_id,
            request_id,
            scope_nonce,
        } => {
            if let Err(error) =
                super::scope_guard::validate_browser_scope_nonce(session, scope_nonce, "open doc")
            {
                ch.send_protocol_error_with_scope_nonce(error, scope_nonce);
                return;
            }
            document::handle_open_doc(state, ch, session, doc_id, request_id).await;
        }
        ClientMessage::RequestHistory {
            doc_id,
            request_id,
            scope_nonce,
        } => {
            if let Err(error) = super::scope_guard::validate_browser_scope_nonce(
                session,
                scope_nonce,
                "document history",
            ) {
                ch.send_protocol_error_with_scope_nonce(error, scope_nonce);
                return;
            }
            document::handle_request_history(state, ch, session, doc_id, request_id).await;
        }
        ClientMessage::Edit {
            doc_id,
            op,
            client_id,
            client_op_id,
            scope_nonce,
        } => {
            if let Err(error) =
                super::scope_guard::validate_browser_scope_nonce(session, scope_nonce, "edit")
            {
                ch.send_protocol_error_with_scope_nonce(error, scope_nonce);
                return;
            }
            document::handle_edit(state, ch, session, doc_id, op, client_id, client_op_id).await;
        }
        ClientMessage::ListDocs {
            request_id,
            scope_nonce,
        } => {
            if let Err(error) = super::scope_guard::validate_browser_scope_nonce(
                session,
                scope_nonce,
                "document list",
            ) {
                ch.send_protocol_error_with_scope_nonce(error, scope_nonce);
                return;
            }
            listing::handle_list_docs(state, ch, session, Some(request_id), None).await;
        }
        ClientMessage::ListShadows {
            request_id,
            scope_nonce,
        } => {
            if let Err(error) = super::scope_guard::validate_browser_scope_nonce(
                session,
                scope_nonce,
                "shadow list",
            ) {
                ch.send_protocol_error_with_scope_nonce(error, scope_nonce);
                return;
            }
            listing::handle_list_shadows(state, ch, Some(session), Some(request_id)).await;
        }
        ClientMessage::ListRepos {
            request_id,
            scope_nonce,
        } => {
            if let Err(error) =
                super::scope_guard::validate_browser_scope_nonce(session, scope_nonce, "repo list")
            {
                ch.send_protocol_error_with_scope_nonce(error, scope_nonce);
                return;
            }
            listing::handle_list_repos(state, ch, session, Some(request_id)).await;
        }
        ClientMessage::Search {
            request_id,
            query,
            limit,
            scope_nonce,
        } => {
            if let Err(error) =
                super::scope_guard::validate_browser_scope_nonce(session, scope_nonce, "search")
            {
                ch.send_protocol_error_with_scope_nonce(error, scope_nonce);
                return;
            }
            search::handle_search(state, ch, request_id, query, limit, scope_nonce).await;
        }
        ClientMessage::PluginCall {
            req_id,
            plugin_id,
            fn_name,
            args,
        } => {
            plugin::handle_plugin_call(state, ch, req_id, plugin_id, fn_name, args).await;
        }
        ClientMessage::SwitchBranch {
            peer_id,
            switch_nonce,
        } => {
            switcher::handle_switch_branch(state, ch, session, peer_id, switch_nonce).await;
        }
        ClientMessage::SwitchRepo { name, switch_nonce } => {
            switcher::handle_switch_repo(state, ch, session, name, None, switch_nonce).await;
        }
        ClientMessage::SwitchRepoExact {
            name,
            repo_id,
            switch_nonce,
        } => {
            switcher::handle_switch_repo(state, ch, session, name, Some(repo_id), switch_nonce)
                .await;
        }
        ClientMessage::DeletePeer {
            peer_id,
            scope_nonce,
        } => {
            if let Err(error) = super::scope_guard::validate_browser_scope_nonce(
                session,
                scope_nonce,
                "delete peer",
            ) {
                ch.send_protocol_error_with_scope_nonce(error, scope_nonce);
                return;
            }
            sync::handle_delete_peer(state, ch, session, peer_id).await;
        }
        ClientMessage::SyncSnapshotRequest { peer_id, repo_id } => {
            sync::handle_sync_snapshot_request(state, ch, session, peer_id, repo_id).await;
        }
        ClientMessage::SyncPushSnapshot {
            peer_id,
            repo_id,
            ops,
        } => {
            sync::handle_sync_push_snapshot(state, ch, session, peer_id, repo_id, ops).await;
        }
        ClientMessage::SyncRequest { repo_id, requests } => {
            sync::handle_sync_request(state, ch, session, repo_id, requests).await;
        }
        ClientMessage::SyncPush { repo_id, ops } => {
            sync::handle_sync_push(state, ch, session, repo_id, ops).await;
        }
        ClientMessage::RegisterWriter {
            peer_id,
            repo_id,
            scope_nonce,
        } => {
            sync::handle_register_writer(ch, session, repo_id, peer_id, scope_nonce);
        }
        ClientMessage::Ping => {
            ch.unicast(deve_core::protocol::ServerMessage::Pong);
        }
        ClientMessage::RequestKey { scope_nonce } => {
            if let Err(error) = super::scope_guard::validate_browser_scope_nonce(
                session,
                scope_nonce,
                "request key",
            ) {
                ch.send_protocol_error_with_scope_nonce(error, scope_nonce);
                return;
            }
            key_exchange::handle_request_key(state, ch, session).await;
        }
        other => {
            tracing::debug!("Unhandled client message: {:?}", other);
        }
    }
}

#[cfg(test)]
#[path = "core_test.rs"]
mod tests;
