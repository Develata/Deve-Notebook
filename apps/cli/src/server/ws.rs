use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use std::sync::Arc;
use futures::{StreamExt, SinkExt}; 

use deve_core::protocol::{ClientMessage, ServerMessage};
use deve_core::models::{LedgerEntry, DocId};
use crate::server::AppState;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    tracing::info!("Client connected");
    
    // 1. Initial Load (Snapshot)
    // Hardcoded doc_id for Phase 3
    let doc_id = DocId::from_u128(1001);
    
    if let Ok(entries_with_seq) = state.ledger.get_ops(doc_id) {
        let ops: Vec<deve_core::models::LedgerEntry> = entries_with_seq.iter().map(|(_, entry)| entry.clone()).collect();
        let content = deve_core::state::reconstruct_content(&ops);
        let version = entries_with_seq.last().map(|(seq, _)| *seq).unwrap_or(0);
        
        let snapshot = ServerMessage::Snapshot { doc_id, content, version };
        if let Ok(json) = serde_json::to_string(&snapshot) {
             let _ = socket.send(Message::Text(json)).await;
        }
    }
    
    // 2. Subscribe to Broadcast
    let mut rx = state.tx.subscribe();
    
    // Split socket
    let (mut sender, mut receiver) = socket.split();
    
    // Task: Receive from Broadcast -> Send to Client
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // Filter: Don't echo back edits to the sender? 
            // Ideally we need ClientId. For now, we trust frontend diffing to handle echoes 
            // or we accept "re-applying" same state is no-op.
            // Actually, if we broadcast NewOps, the client will apply it.
            // If the client *just* sent it, it already applied it.
            // We need to differentiate.
            // Simplified: Verification first.
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Task: Receive from Client -> Persist -> Broadcast
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
                match client_msg {
                    ClientMessage::Edit { doc_id, op, client_id } => {
                       // tracing::info!("Received Edit for Doc {}: {:?}", doc_id, op);
                       
                       let entry = LedgerEntry {
                           doc_id,
                           op: op.clone(),
                           timestamp: chrono::Utc::now().timestamp_millis(),
                       };
                       
                       match state_clone.ledger.append_op(&entry) {
                           Ok(seq) => {
                               // Broadcast to ALL with Sequence and ClientId
                               let _ = tx.send(ServerMessage::NewOp { 
                                   doc_id, 
                                   op, 
                                   seq,
                                   client_id 
                               });
                           }
                           Err(e) => {
                               tracing::error!("Failed to persist op: {:?}", e);
                           }
                       }
                    }
                }
            }
        }
    }
    
    send_task.abort();
    tracing::info!("Client disconnected");
}
