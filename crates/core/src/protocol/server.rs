// crates\core\src\protocol
//! # Server Messages (服务端消息)

use crate::models::{DocId, Op, PeerId, VersionVector};
use crate::security::EncryptedOp;
use crate::source_control::{ChangeEntry, CommitInfo};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    /// 心跳 Pong
    Pong,
    /// 服务端确认操作已持久化
    Ack { doc_id: DocId, seq: u64 },
    /// P2P: 服务端 Hello (响应客户端 Hello)
    SyncHello {
        peer_id: PeerId,
        pub_key: Vec<u8>,
        signature: Vec<u8>,
        vector: VersionVector,
    },
    /// P2P: 服务端向客户端请求数据
    SyncRequest { requests: Vec<(PeerId, (u64, u64))> },
    /// P2P: 服务端向客户端请求快照
    SyncSnapshotRequest {
        peer_id: PeerId,
        repo_id: crate::models::RepoId,
    },
    /// P2P: 服务端推送数据给客户端 (批量)
    SyncPush { ops: Vec<EncryptedOp> },
    /// P2P: 服务端推送快照给客户端
    SyncPushSnapshot {
        peer_id: PeerId,
        repo_id: crate::models::RepoId,
        ops: Vec<EncryptedOp>,
    },

    // === Plugin & AI ===
    /// AI 聊天增量块 (Streaming)
    ChatChunk {
        req_id: String,
        delta: Option<String>,
        finish_reason: Option<String>,
    },

    /// 服务端广播来自其他客户端的新操作
    NewOp {
        doc_id: DocId,
        op: Op,
        seq: u64,
        client_id: u64,
    },
    /// 服务端发送文档完整内容 (初始加载)
    Snapshot {
        doc_id: DocId,
        content: String,
        base_seq: u64,
        version: u64,
        delta_ops: Vec<(u64, Op)>,
    },
    /// 服务端发送完整操作历史 (用于回放)
    History { doc_id: DocId, ops: Vec<(u64, Op)> },
    /// 服务端发送文档列表
    DocList { docs: Vec<(DocId, String)> },
    /// 插件调用响应
    PluginResponse {
        req_id: String,
        result: Option<serde_json::Value>,
        error: Option<String>,
    },
    /// 全文搜索结果
    SearchResults {
        /// (DocId String, Path, Score)
        results: Vec<(String, String, f32)>,
    },

    // === Manual Merge Messages (手动合并模式) ===
    /// 当前同步模式状态
    SyncModeStatus {
        mode: String, // "auto" or "manual"
    },
    /// 待合并操作信息
    PendingOpsInfo {
        count: u32,
        /// 待合并变更预览: (doc_path, old_content_preview, new_content_preview)
        previews: Vec<(String, String, String)>,
    },
    /// 合并完成
    MergeComplete { merged_count: u32 },
    /// 待合并操作已丢弃
    PendingDiscarded,

    // === Branch Switcher Messages (分支切换) ===
    /// 影子库 Peer ID 列表 (远程分支)
    ShadowList { shadows: Vec<String> },
    /// 仓库列表 (当前分支下的 .redb 文件)
    RepoList { repos: Vec<String> },
    /// 分支切换确认
    BranchSwitched {
        peer_id: Option<String>,
        success: bool,
    },
    /// 仓库切换确认
    RepoSwitched { name: String, uuid: String },
    /// 远端 Peer 删除确认
    PeerDeleted { peer_id: String },
    /// 编辑请求被拒绝 (Shadow 分支只读)
    EditRejected { reason: String },

    // === Source Control Responses (版本控制响应) ===
    /// 变更列表响应
    ChangesList {
        /// 已暂存的文件
        staged: Vec<ChangeEntry>,
        /// 未暂存的文件
        unstaged: Vec<ChangeEntry>,
    },
    /// 暂存操作确认
    StageAck { path: String },
    /// 取消暂存确认
    UnstageAck { path: String },
    /// 提交成功响应
    CommitAck {
        /// 提交 ID
        commit_id: String,
        /// 提交时间戳
        timestamp: i64,
    },
    /// 提交历史响应
    CommitHistory { commits: Vec<CommitInfo> },
    /// 文档 Diff 响应 (用于 Diff 视图)
    DocDiff {
        /// 文件路径
        path: String,
        /// 已提交版本内容
        old_content: String,
        /// 当前版本内容
        new_content: String,
    },

    /// 文档删除通知
    DocDeleted { doc_id: DocId },
    /// 放弃变更确认
    DiscardAck { path: String },

    /// 文件树增量更新
    ///
    /// 用于高效同步文件树结构变更。
    TreeUpdate(crate::tree::TreeDelta),

    /// 错误消息
    Error(String),

    /// 批量暂存/取消暂存进度
    ///
    /// 注意: 协议枚举必须追加新变体，避免破坏 bincode 兼容性。
    BulkStageProgress {
        /// "stage" or "unstage"
        op: String,
        total: u32,
        done: u32,
        failed: u32,
    },
    /// 批量暂存/取消暂存完成
    ///
    /// 注意: 协议枚举必须追加新变体，避免破坏 bincode 兼容性。
    BulkStageDone {
        /// "stage" or "unstage"
        op: String,
        total: u32,
        success: u32,
        failed_paths: Vec<String>,
    },

    // === E2EE Key Exchange (密钥交换) ===
    /// 服务端向已认证客户端提供 RepoKey
    ///
    /// **Invariant**: 仅通过已认证的 WSS 通道传输，安全性由 TLS + JWT 保证。
    /// **Post-condition**: 客户端收到后在内存中持有 RepoKey，页面卸载时清除。
    KeyProvide {
        /// AES-256 密钥的原始字节 (32 bytes)
        repo_key: Vec<u8>,
    },
    /// 密钥请求被拒绝 (无认证或服务端无密钥)
    KeyDenied { reason: String },
}
