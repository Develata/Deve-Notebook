use std::sync::Arc;
use tokio::sync::broadcast;
use deve_core::protocol::{ServerMessage, ClientMessage};
use deve_core::models::{PeerId, LedgerEntry};
use deve_core::sync::protocol as sync_proto;
use crate::server::AppState;

/// 处理 P2P 握手请求
pub async fn handle_sync_hello(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    peer_id: PeerId,
    pub_key: Vec<u8>,
    signature: Vec<u8>,
    remote_vector: deve_core::models::VersionVector,
) {
    tracing::info!("Handling SyncHello from {}", peer_id);
    
    // 1. 获取 SyncEngine
    let mut engine = state.sync_engine.write().unwrap();
    let local_peer_id = engine.local_peer_id.clone();
    let local_vector = engine.version_vector().clone();

    // 2. 执行握手逻辑 (Verify Client)
    let result = match engine.handshake(peer_id.clone(), &pub_key, &signature, remote_vector) {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("Handshake failed with {}: {}", peer_id, e);
            return;
        }
    };

    // 3. 构建并发送回执 Hello (Mutual Auth: Sign our response)
    // Msg = "deve-handshake" + local_peer_id + json(local_vector)
    let vec_bytes = serde_json::to_vec(&local_vector).unwrap_or_default();
    let mut msg = Vec::new();
    msg.extend_from_slice(b"deve-handshake");
    msg.extend_from_slice(local_peer_id.as_str().as_bytes());
    msg.extend_from_slice(&vec_bytes);
    
    let my_sig = state.identity_key.sign(&msg);

    let hello_msg = ServerMessage::SyncHello {
        peer_id: local_peer_id,
        pub_key: state.identity_key.public_key_bytes().to_vec(),
        signature: my_sig,
        vector: local_vector,
    };
    let _ = tx.send(hello_msg);
                                
    // 4. 发送请求 (I need data)
    if !result.to_request.is_empty() {
        let requests: Vec<(PeerId, (u64, u64))> = result.to_request.into_iter()
            .map(|req| (req.peer_id, req.range))
            .collect();
            
        let request_msg = ServerMessage::SyncRequest { requests };
        let _ = tx.send(request_msg);
    }

    // 5. 推送数据 (I have data you need)
    // Relay is always-on, so it aggressively pushes what it has found.
    let mut ops_to_push = Vec::new();
    for req in result.to_send {
        if let Ok(response) = engine.get_ops_for_sync(&req) {
            ops_to_push.extend(response.ops);
        }
    }

    if !ops_to_push.is_empty() {
        let push_msg = ServerMessage::SyncPush { ops: ops_to_push };
        let _ = tx.send(push_msg);
    }
}

/// 处理数据请求 (对方想要数据)
pub async fn handle_sync_request(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    requests: Vec<(PeerId, (u64, u64))>,
) {
    let engine = state.sync_engine.read().unwrap();
    let mut ops_to_push = Vec::new();

    for (peer_id, range) in requests {
        let sync_req = sync_proto::SyncRequest {
            peer_id: peer_id,
            range: range,
        };
        
        
        if let Ok(response) = engine.get_ops_for_sync(&sync_req) {
             ops_to_push.extend(response.ops);
        }
    }

    if !ops_to_push.is_empty() {
        let push_msg = ServerMessage::SyncPush { ops: ops_to_push };
        let _ = tx.send(push_msg);
    }
}

/// 处理数据推送 (对方发送数据)
pub async fn handle_sync_push(
    state: &Arc<AppState>,
    _tx: &broadcast::Sender<ServerMessage>,
    peer_id: PeerId, // Added peer_id
    ops: Vec<deve_core::security::EncryptedOp>, // Updated type
) {
    let mut engine = state.sync_engine.write().unwrap();
    
    let response = sync_proto::SyncResponse {
        peer_id: peer_id.clone(),
        ops,
    };

    if let Ok(count) = engine.apply_remote_ops(response) {
        tracing::info!("Applied {} ops from {}", count, peer_id);
    } else {
        tracing::warn!("Failed to apply ops from {}", peer_id);
    }
}
