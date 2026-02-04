// apps/cli/src/server/handlers/sync.rs
//! # P2P 同步消息处理器
//!
//! 处理 P2P 同步相关的消息: SyncHello, SyncRequest, SyncPush

#![allow(dead_code)] // P2P 同步功能尚未完全集成，预留接口

use crate::server::AppState;
use crate::server::channel::DualChannel;
use deve_core::models::PeerId;
use deve_core::protocol::ServerMessage;
use deve_core::sync::protocol as sync_proto;
use std::sync::Arc;

/// 处理 P2P 握手请求
pub async fn handle_sync_hello(
    state: &Arc<AppState>,
    ch: &DualChannel,
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
            // 使用单播发送错误
            ch.send_error(format!("Handshake failed: {}", e));
            return;
        }
    };

    // 3. 构建并发送回执 Hello (Mutual Auth: Sign our response)
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
    // 单播回复给发起方
    ch.unicast(hello_msg);

    // 4. 发送请求 (I need data)
    if !result.to_request.is_empty() {
        let requests: Vec<(PeerId, (u64, u64))> = result
            .to_request
            .into_iter()
            .map(|req| (req.peer_id, req.range))
            .collect();

        let request_msg = ServerMessage::SyncRequest { requests };
        ch.unicast(request_msg);
    }

    // 4.1 发送快照请求 (I need snapshot)
    for req in result.snapshot_requests {
        let msg = ServerMessage::SyncSnapshotRequest {
            peer_id: req.peer_id,
            repo_id: req.repo_id,
        };
        ch.unicast(msg);
    }

    // 5. 推送数据 (I have data you need)

    // 5. 推送数据 (I have data you need)
    let mut ops_to_push = Vec::new();
    for req in result.to_send {
        if let Ok(response) = engine.get_ops_for_sync(&req) {
            ops_to_push.extend(response.ops);
        }
    }

    if !ops_to_push.is_empty() {
        let push_msg = ServerMessage::SyncPush { ops: ops_to_push };
        ch.unicast(push_msg);
    }
}

/// 处理数据请求 (对方想要数据)
pub async fn handle_sync_request(
    state: &Arc<AppState>,
    ch: &DualChannel,
    requests: Vec<(PeerId, (u64, u64))>,
) {
    let engine = state.sync_engine.read().unwrap();
    let mut ops_to_push = Vec::new();

    for (peer_id, range) in requests {
        let repo_id = super::get_repo_id(state);
        let sync_req = sync_proto::SyncRequest {
            peer_id,
            repo_id,
            range,
        };

        if let Ok(response) = engine.get_ops_for_sync(&sync_req) {
            ops_to_push.extend(response.ops);
        }
    }

    if !ops_to_push.is_empty() {
        let push_msg = ServerMessage::SyncPush { ops: ops_to_push };
        ch.unicast(push_msg);
    }
}

/// 处理数据推送 (对方发送数据)
pub async fn handle_sync_push(
    state: &Arc<AppState>,
    ch: &DualChannel,
    peer_id: PeerId,
    ops: Vec<deve_core::security::EncryptedOp>,
) {
    let mut engine = state.sync_engine.write().unwrap();

    let repo_id = super::get_repo_id(state);
    let response = sync_proto::SyncResponse {
        peer_id: peer_id.clone(),
        repo_id,
        ops,
    };

    match engine.apply_remote_ops(response) {
        Ok(count) => {
            tracing::info!("Applied {} ops from {}", count, peer_id);
        }
        Err(e) => {
            tracing::error!("Failed to apply ops from {}: {:?}", peer_id, e);
            // 使用单播发送错误给当前客户端
            ch.send_error(format!("Failed to apply sync ops from {}: {}", peer_id, e));
        }
    }
}

/// 处理快照请求 (对方落后太多，请求全量)
pub async fn handle_sync_snapshot_request(
    state: &Arc<AppState>,
    ch: &DualChannel,
    peer_id: PeerId,
    repo_id: deve_core::models::RepoId,
) {
    let engine = state.sync_engine.read().unwrap();
    tracing::info!("Handling SnapshotRequest from {}", peer_id);

    let request = deve_core::sync::protocol::SyncSnapshotRequest {
        peer_id: peer_id.clone(),
        repo_id,
    };

    match engine.get_snapshot_for_sync(&request) {
        Ok(response) => {
            tracing::info!(
                "Sending snapshot with {} ops to {}",
                response.ops.len(),
                peer_id
            );
            let msg = ServerMessage::SyncPushSnapshot {
                peer_id: engine.local_peer_id.clone(), // I am the source
                repo_id: response.repo_id,
                ops: response.ops,
            };
            ch.unicast(msg);
        }
        Err(e) => {
            tracing::error!("Failed to generate snapshot for {}: {:?}", peer_id, e);
            ch.send_error(format!("Failed to generate snapshot: {}", e));
        }
    }
}

/// 处理快照推送 (对方发送全量数据)
pub async fn handle_sync_push_snapshot(
    state: &Arc<AppState>,
    ch: &DualChannel,
    peer_id: PeerId,
    repo_id: deve_core::models::RepoId,
    ops: Vec<deve_core::security::EncryptedOp>,
) {
    let mut engine = state.sync_engine.write().unwrap();
    tracing::info!("Handling PushSnapshot from {} ({} ops)", peer_id, ops.len());

    let response = deve_core::sync::protocol::SyncResponse {
        peer_id: peer_id.clone(),
        repo_id,
        ops,
    };

    match engine.apply_remote_snapshot(response) {
        Ok(seq) => {
            tracing::info!(
                "Applied snapshot from {}. Updated VV to seq {}",
                peer_id,
                seq
            );
        }
        Err(e) => {
            tracing::error!("Failed to apply snapshot from {}: {:?}", peer_id, e);
            ch.send_error(format!("Failed to apply snapshot: {}", e));
        }
    }
}

/// 处理删除 Peer 请求 (物理删除远端分支)
pub async fn handle_delete_peer(state: &Arc<AppState>, ch: &DualChannel, peer_id_str: String) {
    let peer_id = PeerId::new(peer_id_str.clone());
    tracing::info!("Handling DeletePeer request for: {}", peer_id);

    // 1. 调用 RepoManager 执行物理删除
    match state.repo.delete_peer_branch(&peer_id) {
        Ok(_) => {
            tracing::info!("Successfully deleted peer branch: {}", peer_id);

            // 2. 发送确认消息
            ch.broadcast(ServerMessage::PeerDeleted {
                peer_id: peer_id_str,
            });

            // 3. 广播最新的 Shadow 列表 (刷新所有客户端侧边栏)
            crate::server::handlers::listing::handle_list_shadows(state, ch).await;
        }
        Err(e) => {
            tracing::error!("Failed to delete peer branch {}: {:?}", peer_id, e);
            ch.send_error(format!("Failed to delete peer: {}", e));
        }
    }
}
