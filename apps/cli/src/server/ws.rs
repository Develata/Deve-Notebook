use axum::extract::State;
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::{
    docs, document, listing, merge, plugin, search, source_control, switcher, sync,
};
use crate::server::session::WsSession;
use deve_core::protocol::{ClientMessage, ServerMessage};

/// HTTP/WebSocket 入口
pub async fn ws_handler(
    ws: axum::extract::WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let peer_id = uuid::Uuid::new_v4().to_string();
    ws.on_upgrade(move |socket| handle_socket(state, socket, peer_id))
}

/// WebSocket 消息处理器
pub async fn handle_socket(
    state: Arc<AppState>,
    socket: axum::extract::ws::WebSocket,
    peer_id: String,
) {
    let (mut sender, mut receiver) = socket.split();

    // Create Unicast Channel (MPSC)
    let (unicast_tx, mut unicast_rx) = mpsc::unbounded_channel::<ServerMessage>();

    tokio::spawn(async move {
        while let Some(msg) = unicast_rx.recv().await {
            if let Ok(text) = serde_json::to_string(&msg) {
                if let Err(e) = sender.send(axum::extract::ws::Message::Text(text)).await {
                    tracing::warn!("Failed to send message to WS: {:?}", e);
                    break;
                }
            }
        }
    });

    // Create DualChannel
    let ch = DualChannel::new(state.tx.clone(), unicast_tx.clone());

    tracing::info!("Client connected: {}", peer_id);

    // 会话状态
    let mut session = WsSession::new();

    // 主循环
    while let Some(msg) = receiver.next().await {
        if let Ok(msg) = msg {
            match msg {
                axum::extract::ws::Message::Text(text) => {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                        route_message(&state, &ch, &mut session, client_msg).await;
                    } else {
                        tracing::warn!("Failed to parse client message: {}", text);
                    }
                }
                axum::extract::ws::Message::Binary(bin) => {
                    tracing::warn!("Received binary message (ignored): {} bytes", bin.len());
                }
                axum::extract::ws::Message::Close(_) => {
                    tracing::info!("Client disconnected: {}", peer_id);
                    break;
                }
                _ => {}
            }
        } else {
            break;
        }
    }
}

/// 消息路由
async fn route_message(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    msg: ClientMessage,
) {
    match msg {
        // === 握手与初始化 ===
        ClientMessage::SyncHello {
            peer_id,
            pub_key,
            signature,
            vector,
        } => {
            sync::handle_sync_hello(state, ch, peer_id, pub_key, signature, vector).await;
        }

        // === 文档操作 ===
        ClientMessage::CreateDoc { name } => {
            docs::handle_create_doc(state, ch, session, name).await;
        }
        ClientMessage::RenameDoc { old_path, new_path } => {
            docs::handle_rename_doc(state, ch, session, old_path, new_path).await;
        }
        ClientMessage::DeleteDoc { path } => {
            docs::handle_delete_doc(state, ch, session, path).await;
        }
        ClientMessage::CopyDoc {
            src_path,
            dest_path,
        } => {
            docs::handle_copy_doc(state, ch, session, src_path, dest_path).await;
        }
        ClientMessage::MoveDoc {
            src_path,
            dest_path,
        } => {
            docs::handle_move_doc(state, ch, session, src_path, dest_path).await;
        }

        // === 编辑与同步 ===
        ClientMessage::OpenDoc { doc_id } => {
            document::handle_open_doc(state, ch, session, doc_id).await;
        }
        ClientMessage::Edit { doc_id, op, .. } => {
            document::handle_edit(state, ch, session, doc_id, op, 0).await;
        }

        // === 列表查询 ===
        ClientMessage::ListDocs => {
            listing::handle_list_docs(state, ch, session).await;
        }
        ClientMessage::ListShadows => {
            listing::handle_list_shadows(state, ch).await;
        }
        ClientMessage::ListRepos => {
            listing::handle_list_repos(state, ch, session.active_branch.as_ref()).await;
        }

        // === 合并操作 ===
        ClientMessage::GetSyncMode => {
            merge::handle_get_sync_mode(state, ch).await;
        }
        ClientMessage::SetSyncMode { mode } => {
            merge::handle_set_sync_mode(state, ch, mode).await;
        }
        ClientMessage::GetPendingOps => {
            merge::handle_get_pending_ops(state, ch).await;
        }
        ClientMessage::ConfirmMerge => {
            merge::handle_confirm_merge(state, ch).await;
        }
        ClientMessage::DiscardPending => {
            merge::handle_discard_pending(state, ch).await;
        }
        ClientMessage::MergePeer { peer_id, doc_id } => {
            merge::handle_merge_peer(state, ch, peer_id, doc_id).await;
        }

        // === 搜索与插件 ===
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

        // === 分支切换 (Updated to safe handler) ===
        ClientMessage::SwitchBranch { peer_id } => {
            switcher::handle_switch_branch(state, ch, session, peer_id).await;
        }
        ClientMessage::SwitchRepo { name } => {
            switcher::handle_switch_repo(state, ch, session, name).await;
        }
        ClientMessage::DeletePeer { peer_id } => {
            sync::handle_delete_peer(state, ch, peer_id).await;
        }

        // === 版本控制 ===

        ClientMessage::GetChanges => {
            source_control::handle_get_changes(state, ch, session).await;
        }
        ClientMessage::StageFile { path } => {
            source_control::handle_stage_file(state, ch, path).await;
        }
        ClientMessage::UnstageFile { path } => {
            source_control::handle_unstage_file(state, ch, path).await;
        }
        ClientMessage::DiscardFile { path } => {
            source_control::handle_discard_file(state, ch, session, path).await;
        }
        ClientMessage::Commit { message } => {
            source_control::handle_commit(state, ch, message).await;
        }
        ClientMessage::GetCommitHistory { limit } => {
            source_control::handle_get_commit_history(state, ch, limit as u32).await;
        }
        ClientMessage::GetDocDiff { path } => {
            source_control::handle_get_doc_diff(state, ch, session, path).await;
        }

        _ => {}
    }
}
