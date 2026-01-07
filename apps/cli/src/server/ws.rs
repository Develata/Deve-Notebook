use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use std::sync::Arc;
use deve_core::ledger::Ledger;

pub struct AppState {
    pub ledger: Arc<Ledger>,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

use deve_core::protocol::{ClientMessage, ServerMessage};
use deve_core::models::LedgerEntry;

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    tracing::info!("Client connected");
    while let Some(msg) = socket.recv().await {
        let msg = if let Ok(msg) = msg {
            msg
        } else {
            // client disconnected
            return;
        };

        if let Message::Text(text) = msg {
            // tracing::debug!("Received message: {}", text);
            
            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                match client_msg {
                    ClientMessage::Edit { doc_id, op } => {
                       tracing::info!("Received Edit for Doc {}: {:?}", doc_id, op);
                       
                       let entry = LedgerEntry {
                           doc_id,
                           op,
                           timestamp: chrono::Utc::now().timestamp_millis(),
                       };
                       
                       match state.ledger.append_op(&entry) {
                           Ok(seq) => {
                               tracing::info!("Persisted op at seq {}", seq);
                               let ack = ServerMessage::Ack { doc_id, seq };
                               if let Ok(ack_json) = serde_json::to_string(&ack) {
                                   let _ = socket.send(Message::Text(ack_json)).await;
                               }
                           }
                           Err(e) => {
                               tracing::error!("Failed to persist op: {:?}", e);
                           }
                       }
                    }
                }
            } else {
                tracing::warn!("Failed to deserialize message: {}", text);
            }
        }
    }
    tracing::info!("Client disconnected");
}
