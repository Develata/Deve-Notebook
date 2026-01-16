//! # P2P 同步协议定义 (P2P Sync Protocol)
//!
//! **架构作用**:
//! 定义 P2P 同步协议的消息结构与核心转换逻辑。
//! 负责将 Version Vector 的差异转换为具体的同步请求。
//!
//! **核心功能清单**:
//! - `SyncRequest`: 同步请求消息。
//! - `SyncResponse`: 同步响应消息。
//! - `HandshakeResult`: 握手结果。
//! - `compute_diff_requests`: 计算差异并生成请求列表。
//!
//! **类型**: Core MUST (核心必选)

use crate::models::{PeerId, LedgerEntry};
use crate::sync::vector::VersionVector;
use crate::ledger::repo_type::RepoId;

/// 同步请求：表示需要从某个 Peer 拉取的数据范围
#[derive(Debug, Clone)]
pub struct SyncRequest {
    /// 目标 Peer ID
    pub peer_id: PeerId,
    /// 目标仓库 ID
    pub repo_id: RepoId,
    /// 需要拉取的序列号范围 (start, end) - 左闭右开
    pub range: (u64, u64),
}

use crate::security::EncryptedOp;

/// 同步响应：包含拉取到的加密操作列表
#[derive(Debug, Clone)]
pub struct SyncResponse {
    /// 来源 Peer ID
    pub peer_id: PeerId,
    /// 来源仓库 ID
    pub repo_id: RepoId,
    /// 加密的操作列表 (Envelope Body)
    pub ops: Vec<EncryptedOp>,
}

/// 握手结果
#[derive(Debug)]
pub struct HandshakeResult {
    /// 需要发送给远端的数据范围
    pub to_send: Vec<SyncRequest>,
    /// 需要从远端请求的数据范围
    pub to_request: Vec<SyncRequest>,
    /// 是否自动模式（Auto 模式会自动应用）
    pub auto_apply: bool,
}

/// 基于 Version Vector 计算差异，并生成同步请求列表
/// 
/// 返回:
/// - `to_send`: 需要发送给远端的数据范围 (远端缺失)
/// - `to_request`: 需要从远端请求的数据范围 (本地缺失)
pub fn compute_diff_requests(
    local_vector: &VersionVector, 
    remote_vector: &VersionVector
) -> (Vec<SyncRequest>, Vec<SyncRequest>) {
    let (missing_from_remote, missing_from_local) = local_vector.diff(remote_vector);
    
    // TODO: Acquire current active RepodId context?
    // For now we assume default repo ID (nil) for backward compatibility
    let default_repo_id = uuid::Uuid::nil();

    let to_send: Vec<SyncRequest> = missing_from_remote
        .into_iter()
        .map(|(peer_id, range)| SyncRequest {
            peer_id,
            repo_id: default_repo_id,
            range: (range.start, range.end),
        })
        .collect();

    let to_request: Vec<SyncRequest> = missing_from_local
        .into_iter()
        .map(|(peer_id, range)| SyncRequest {
            peer_id,
            repo_id: default_repo_id,
            range: (range.start, range.end),
        })
        .collect();

    (to_send, to_request)
}
