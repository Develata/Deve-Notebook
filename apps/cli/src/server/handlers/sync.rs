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
    remote_vector: deve_core::models::VersionVector,
) {
    tracing::info!("Handling SyncHello from {}", peer_id);
    
    // 1. 获取 SyncEngine
    let mut engine = state.sync_engine.write().unwrap();
    let local_peer_id = engine.local_peer_id.clone();
    let local_vector = engine.version_vector().clone();

    // 2. 执行握手逻辑
    let result = engine.handshake(peer_id.clone(), remote_vector);

    // 3. 构建并发送回执 Hello (让对方也能计算差异)
    let hello_msg = ServerMessage::SyncHello {
        peer_id: local_peer_id,
        vector: local_vector,
    };
    let _ = tx.send(hello_msg); // Broadcast? No, should be unicast usually, but currently we broadcast to all WS.
                                // Improvement: Unicast to this socket. 
                                // But current architecture broadcasts everything.
                                // For now, broadcast is acceptable for Relay (assuming small scale or filtered by client).
                                
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
    tx: &broadcast::Sender<ServerMessage>,
    peer_id: PeerId, // Added peer_id
    ops: Vec<(u64, LedgerEntry)>, // Fixed type
) {
    let mut engine = state.sync_engine.write().unwrap();
    
    let response = sync_proto::SyncResponse {
        peer_id: peer_id.clone(),
        ops,
    };

    if let Ok(count) = engine.apply_remote_ops(response) {
        tracing::info!("Applied {} ops from {}", count, peer_id);
        
        // Broadcast NewOp to watchers? 
        // NOTE: NewOp is currently single Op. For logic simplicity we might skip broadcasting individual ops 
        // if we assume watchers will periodic refresh or use another signal.
        // But for "Live" collaboration, we should broadcast.
        // However, we don't have the `Op` objects easily accessible here without iterating again? 
        // We moved `ops` into `response`.
        // Let's rely on SyncEngine functionality.
        // Ideally, SyncEngine or RepoManager should emit events. 
        // But for now, let's keep it simple.
        // We will just log.
    }
}
