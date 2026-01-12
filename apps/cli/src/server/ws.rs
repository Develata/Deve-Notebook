//! # WebSocket Handler (WebSocket 连接处理器)
//!
//! **架构作用**:
//! 处理 WebSocket 连接升级、生命周期管理及消息路由。
//!
//! **核心功能清单**:
//! - `ws_handler`: Axum 路由处理器，升级 HTTP 到 WebSocket。
//! - `handle_socket`: 连接主循环。
//!   - 接收 ClientMessage 并路由到 api/system handlers。
//!   - 订阅 ServerMessage 广播并推送到客户端。
//!
//! **类型**: Core MUST (核心必选)
//!
//! 消息根据类型路由到 `handlers::document` 或 `handlers::system`。

use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use std::sync::Arc;
use futures::{StreamExt, SinkExt}; 

use deve_core::protocol::{ClientMessage, ServerMessage};
use crate::server::AppState;
use crate::server::handlers::{document, system, plugin, search, sync};
use deve_core::models::PeerId;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    tracing::info!("Client connected");
    
    // Session State
    let mut authenticated_peer_id: Option<PeerId> = None;
    
    // Subscribe to Broadcast
    let mut rx = state.tx.subscribe();
    
    // Direct Message Channel (Unicast)
    let (direct_tx, mut direct_rx) = tokio::sync::mpsc::unbounded_channel::<ServerMessage>();

    // Split socket
    let (mut sender, mut receiver) = socket.split();
    
    // Task: Receive from Broadcast OR Direct -> Send to Client
    let send_task = tokio::spawn(async move {
        loop {
            let msg_to_send = tokio::select! {
                res = rx.recv() => {
                    match res {
                        Ok(msg) => msg,
                        Err(_e) => continue, // Broadcast lag or closed
                    }
                },
                res = direct_rx.recv() => {
                    match res {
                        Some(msg) => msg,
                        None => break, // Channel closed
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

    // Task: Receive from Client -> Route to Handlers
    let state_clone = state.clone();
    let tx = state.tx.clone();
    
    while let Some(msg) = receiver.next().await {
        let msg = if let Ok(msg) = msg {
            msg
        } else {
            return;
        };

        if let Message::Text(text) = msg {
            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                // ROUTER
                match client_msg {
                     ClientMessage::Ping => {
                         // Unicast Pong
                         let _ = direct_tx.send(ServerMessage::Pong);
                     }
                     ClientMessage::Edit { doc_id, op, client_id } => {
                         document::handle_edit(&state_clone, &tx, doc_id, op, client_id).await;
                     }
                     ClientMessage::RequestHistory { doc_id } => {
                         document::handle_request_history(&state_clone, &tx, doc_id).await;
                     }
                     ClientMessage::OpenDoc { doc_id } => {
                         document::handle_open_doc(&state_clone, &tx, doc_id).await;
                     }
                     ClientMessage::ListDocs => {
                         system::handle_list_docs(&state_clone, &tx).await;
                     }
                     ClientMessage::CreateDoc { name } => {
                         system::handle_create_doc(&state_clone, &tx, name).await;
                     }
                     ClientMessage::RenameDoc { old_path, new_path } => {
                         system::handle_rename_doc(&state_clone, &tx, old_path, new_path).await;
                     }
                     ClientMessage::DeleteDoc { path } => {
                         system::handle_delete_doc(&state_clone, &tx, path).await;
                     }
                     ClientMessage::CopyDoc { src_path, dest_path } => {
                         system::handle_copy_doc(&state_clone, &tx, src_path, dest_path).await;
                     }
                     ClientMessage::MoveDoc { src_path, dest_path } => {
                         system::handle_move_doc(&state_clone, &tx, src_path, dest_path).await;
                     }
                     ClientMessage::PluginCall { req_id, plugin_id, fn_name, args } => {
                         plugin::handle_plugin_call(&state_clone, &tx, req_id, plugin_id, fn_name, args).await;
                     }
                     ClientMessage::Search { query, limit } => {
                         search::handle_search(&state_clone, &tx, query, limit).await;
                     }
                     // P2P Sync messages
                     ClientMessage::SyncHello { peer_id, vector } => {
                         tracing::info!("SyncHello from {}", peer_id);
                         authenticated_peer_id = Some(peer_id.clone());
                         sync::handle_sync_hello(&state_clone, &tx, peer_id, vector).await;
                     }
                     ClientMessage::SyncRequest { requests } => {
                         sync::handle_sync_request(&state_clone, &tx, requests).await;
                     }
                     ClientMessage::SyncPush { ops } => {
                         if let Some(pid) = &authenticated_peer_id {
                             sync::handle_sync_push(&state_clone, &tx, pid.clone(), ops).await;
                         } else {
                             tracing::warn!("Ignored SyncPush from unauthenticated peer");
                         }
                     }
                }
            }
        }
    }
    
    send_task.abort();
    tracing::info!("Client disconnected");
}
