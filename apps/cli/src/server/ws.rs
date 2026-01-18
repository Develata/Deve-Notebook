// apps/cli/src/server/ws.rs
//! # WebSocket Handler (WebSocket 连接处理器)
//!
//! **架构作用**:
//! 处理 WebSocket 连接升级、生命周期管理及消息路由。
//!
//! **核心功能**:
//! - `ws_handler`: Axum 路由处理器，升级 HTTP 到 WebSocket
//! - `handle_socket`: 连接主循环
//!
//! **类型**: Core MUST (核心必选)

use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::{
    docs, document, listing, merge, plugin, search, source_control, sync,
};
use crate::server::session::WsSession;
use deve_core::protocol::{ClientMessage, ServerMessage};

/// Axum WebSocket 升级处理器
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// WebSocket 连接主循环
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    tracing::info!("Client connected");

    // 会话状态
    let mut session = WsSession::new();

    // 订阅广播通道
    let mut rx = state.tx.subscribe();

    // 创建单播通道
    let (direct_tx, mut direct_rx) = tokio::sync::mpsc::unbounded_channel::<ServerMessage>();

    // 分离 Socket
    let (mut sender, mut receiver) = socket.split();

    // Task: 从广播/单播接收消息并发送到客户端
    let send_task = tokio::spawn(async move {
        loop {
            let msg_to_send = tokio::select! {
                res = rx.recv() => {
                    match res {
                        Ok(msg) => msg,
                        Err(_) => continue,
                    }
                },
                res = direct_rx.recv() => {
                    match res {
                        Some(msg) => msg,
                        None => break,
                    }
                }
            };

            if let Ok(json) = serde_json::to_string(&msg_to_send) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // 创建双通道上下文
    let ch = DualChannel::new(state.tx.clone(), direct_tx);

    // 主循环: 接收客户端消息并路由
    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(_) => break,
        };

        if let Message::Text(text) = msg {
            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                route_message(&state, &ch, &mut session, client_msg).await;
            }
        }
    }

    send_task.abort();
    tracing::info!("Client disconnected");
}

/// 消息路由器
///
/// 根据消息类型分发到对应的 Handler
async fn route_message(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    msg: ClientMessage,
) {
    match msg {
        // === 基础消息 ===
        ClientMessage::Ping => {
            ch.unicast(ServerMessage::Pong);
        }

        // === 文档操作 ===
        ClientMessage::Edit {
            doc_id,
            op,
            client_id,
        } => {
            if session.is_readonly() {
                ch.send_error("Cannot edit on shadow branch (read-only)".to_string());
            } else {
                document::handle_edit(state, ch, doc_id, op, client_id).await;
            }
        }
        ClientMessage::RequestHistory { doc_id } => {
            document::handle_request_history(state, ch, doc_id).await;
        }
        ClientMessage::OpenDoc { doc_id } => {
            document::handle_open_doc(state, ch, doc_id, session.active_branch.as_ref()).await;
        }

        // === 列表查询 ===
        ClientMessage::ListDocs => {
            listing::handle_list_docs(state, ch, session.active_branch.as_ref()).await;
        }
        ClientMessage::ListShadows => {
            listing::handle_list_shadows(state, ch).await;
        }
        ClientMessage::ListRepos => {
            listing::handle_list_repos(state, ch, session.active_branch.as_ref()).await;
        }

        // === 文档 CRUD ===
        ClientMessage::CreateDoc { name } => {
            docs::handle_create_doc(state, ch, name).await;
        }
        ClientMessage::RenameDoc { old_path, new_path } => {
            docs::handle_rename_doc(state, ch, old_path, new_path).await;
        }
        ClientMessage::DeleteDoc { path } => {
            docs::handle_delete_doc(state, ch, path).await;
        }
        ClientMessage::CopyDoc {
            src_path,
            dest_path,
        } => {
            docs::handle_copy_doc(state, ch, src_path, dest_path).await;
        }
        ClientMessage::MoveDoc {
            src_path,
            dest_path,
        } => {
            docs::handle_move_doc(state, ch, src_path, dest_path).await;
        }

        // === P2P 同步 ===
        ClientMessage::SyncHello {
            peer_id,
            pub_key,
            signature,
            vector,
        } => {
            tracing::info!("SyncHello from {}", peer_id);
            session.set_authenticated(peer_id.clone());
            sync::handle_sync_hello(state, ch, peer_id, pub_key, signature, vector).await;
        }
        ClientMessage::SyncRequest { requests } => {
            sync::handle_sync_request(state, ch, requests).await;
        }
        ClientMessage::SyncPush { ops } => {
            if let Some(pid) = &session.authenticated_peer_id {
                sync::handle_sync_push(state, ch, pid.clone(), ops).await;
            } else {
                tracing::warn!("Ignored SyncPush from unauthenticated peer");
            }
        }

        // === 手动合并 ===
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

        // === 分支切换 ===
        ClientMessage::SwitchBranch { peer_id } => {
            session.switch_branch(peer_id.clone());
            tracing::info!("Client switched to branch: {:?}", session.active_branch);
            ch.unicast(ServerMessage::BranchSwitched {
                peer_id,
                success: true,
            });
            // 刷新列表
            listing::handle_list_docs(state, ch, session.active_branch.as_ref()).await;
            listing::handle_list_repos(state, ch, session.active_branch.as_ref()).await;
        }

        // === 版本控制 ===
        ClientMessage::GetChanges => {
            source_control::handle_get_changes(state, ch).await;
        }
        ClientMessage::StageFile { path } => {
            source_control::handle_stage_file(state, ch, path).await;
        }
        ClientMessage::UnstageFile { path } => {
            source_control::handle_unstage_file(state, ch, path).await;
        }
        ClientMessage::Commit { message } => {
            source_control::handle_commit(state, ch, message).await;
        }
        ClientMessage::GetCommitHistory { limit } => {
            source_control::handle_get_commit_history(state, ch, limit).await;
        }
        ClientMessage::GetDocDiff { path } => {
            source_control::handle_get_doc_diff(state, ch, path).await;
        }

        // === 插件 ===
        ClientMessage::PluginCall {
            req_id,
            plugin_id,
            fn_name,
            args,
        } => {
            plugin::handle_plugin_call(state, ch, req_id, plugin_id, fn_name, args).await;
        }

        // === 搜索 ===
        ClientMessage::Search { query, limit } => {
            search::handle_search(state, ch, query, limit).await;
        }
    }
}
