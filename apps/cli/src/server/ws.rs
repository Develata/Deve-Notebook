//! # WebSocket 连接处理器
//!
//! 本模块管理单个 WebSocket 连接并路由消息。
//!
//! ## 处理流程
//!
//! 1. `ws_handler`: 将 HTTP 升级为 WebSocket
//! 2. `handle_socket`: 管理连接生命周期
//!    - 创建任务接收广播消息并发送给客户端
//!    - 循环接收客户端消息并路由到相应处理器
//!
//! 消息根据类型路由到 `handlers::document` 或 `handlers::system`。

use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use std::sync::Arc;
use futures::{StreamExt, SinkExt}; 

use deve_core::protocol::ClientMessage;
use crate::server::AppState;
use crate::server::handlers::{document, system};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    tracing::info!("Client connected");
    
    // Subscribe to Broadcast
    let mut rx = state.tx.subscribe();
    
    // Split socket
    let (mut sender, mut receiver) = socket.split();
    
    // Task: Receive from Broadcast -> Send to Client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
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
                }
            }
        }
    }
    
    send_task.abort();
    tracing::info!("Client disconnected");
}
