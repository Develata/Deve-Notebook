use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::StreamExt;
use std::sync::Arc;

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::protocol::ClientMessage;

mod route;
pub(crate) mod send;

/// HTTP/WebSocket 入口。
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let peer_id = uuid::Uuid::new_v4().to_string();
    ws.on_upgrade(move |socket| handle_socket(state, socket, peer_id))
}

/// WebSocket 消息处理器。
pub async fn handle_socket(state: Arc<AppState>, socket: axum::extract::ws::WebSocket, peer_id: String) {
    let (sender, mut receiver) = socket.split();

    // 为每个连接创建有界单播队列，避免慢客户端导致无界内存增长。
    let (unicast_tx, unicast_rx) = send::new_unicast_channel();

    // 将单播队列写入 WebSocket。
    send::spawn_unicast_sender_task(sender, unicast_rx);

    // 订阅广播并尝试转发到单播队列（带背压/丢弃策略）。
    let broadcast_rx = state.tx.subscribe();
    send::spawn_broadcast_forwarder(broadcast_rx, unicast_tx.clone());

    let ch = DualChannel::new(state.tx.clone(), unicast_tx);

    tracing::info!("Client connected: {}", peer_id);

    let mut session = WsSession::new();

    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                tracing::warn!("WS receive error: {:?}", e);
                break;
            }
        };

        match msg {
            axum::extract::ws::Message::Text(text) => {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => route::route_message(&state, &ch, &mut session, client_msg).await,
                    Err(_) => tracing::warn!("Failed to parse client message: {}", text),
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
    }
}
