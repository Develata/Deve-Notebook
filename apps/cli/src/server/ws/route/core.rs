use crate::server::handlers::{document, listing, plugin, search, switcher, sync};
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
        ClientMessage::OpenDoc { doc_id } => {
            document::handle_open_doc(state, ch, session, doc_id).await;
        }
        ClientMessage::Edit {
            doc_id,
            op,
            client_id,
        } => {
            document::handle_edit(state, ch, session, doc_id, op, client_id).await;
        }
        ClientMessage::ListDocs => {
            listing::handle_list_docs(state, ch, session).await;
        }
        ClientMessage::ListShadows => {
            listing::handle_list_shadows(state, ch).await;
        }
        ClientMessage::ListRepos => {
            listing::handle_list_repos(state, ch, session.active_branch.as_ref()).await;
        }
        ClientMessage::Search { query, limit } => {
            search::handle_search(state, ch, query, limit).await;
        }
        ClientMessage::PluginCall {
            req_id,
            plugin_id,
            fn_name,
            args,
        } => {
            plugin::handle_plugin_call(state, ch, req_id, plugin_id, fn_name, args).await;
        }
        ClientMessage::SwitchBranch { peer_id } => {
            switcher::handle_switch_branch(state, ch, session, peer_id).await;
        }
        ClientMessage::SwitchRepo { name } => {
            switcher::handle_switch_repo(state, ch, session, name).await;
        }
        ClientMessage::DeletePeer { peer_id } => {
            sync::handle_delete_peer(state, ch, peer_id).await;
        }
        ClientMessage::SyncSnapshotRequest { peer_id, repo_id } => {
            sync::handle_sync_snapshot_request(state, ch, peer_id, repo_id).await;
        }
        ClientMessage::SyncPushSnapshot {
            peer_id,
            repo_id,
            ops,
        } => {
            sync::handle_sync_push_snapshot(state, ch, peer_id, repo_id, ops).await;
        }
        ClientMessage::Ping => {
            ch.unicast(deve_core::protocol::ServerMessage::Pong);
        }
        other => {
            tracing::debug!("Unhandled client message: {:?}", other);
        }
    }
}
