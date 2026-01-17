//! # WebSocket Protocol (通信协议)
//!
//! **架构作用**:
//! 定义客户端与服务端之间的 WebSocket 通信消息格式。
//!
//! **核心功能清单**:
//! - `ClientMessage`: 定义客户端发起的请求（Edit, List, Open, Create, Copy, Move, Delete 等）。
//! - `ServerMessage`: 定义服务端推送的响应与事件（DocList, Snapshot, NewOps, Error 等）。
//! - `Op`: 定义 CRDT 操作单元。
//!
//! **类型**: Core MUST (核心必选)
//!
//! - `ServerMessage`: 服务端发送给客户端的消息
//!   - Ack（确认）, NewOp（新操作）, Snapshot（快照）
//!   - History（历史）, DocList（文档列表）, Error（错误）

use serde::{Serialize, Deserialize};
use crate::models::{DocId, Op, PeerId, LedgerEntry, VersionVector};
use crate::source_control::{ChangeEntry, CommitInfo};
use crate::security::EncryptedOp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    /// 心跳 Ping
    Ping,
    /// P2P 握手请求 (Hello)
    ///
    /// **参数**:
    /// - `peer_id`: 发起方节点 ID。
    /// - `pub_key`: 发起方身份公钥 (Ed25519)。
    /// - `signature`: 握手签名 (防止伪造)。
    /// - `vector`: 发起方当前的 Version Vector。
    SyncHello {
        peer_id: PeerId,
        pub_key: Vec<u8>,
        signature: Vec<u8>,
        vector: VersionVector,
    },
    /// 请求缺失的操作记录
    SyncRequest {
        /// 请求列表 [(PeerId, StartSeq, EndSeq)]
        /// 注意: Range<u64> 默认不可序列化，因此使用 (u64, u64) 元组。
        requests: Vec<(PeerId, (u64, u64))>,
    },
    /// 推送加密操作记录给对端 (Envelope Mode)
    SyncPush {
        ops: Vec<EncryptedOp>,
    },
    /// 客户端发送编辑操作 (针对特定文档)
    Edit {
        doc_id: DocId,
        op: Op,
        client_id: u64,
    },
    /// 请求文档的完整操作历史
    RequestHistory {
        doc_id: DocId,
    },
    /// 请求所有已知文档的列表
    ListDocs,
    /// 请求打开指定文档 (获取快照)
    OpenDoc {
        doc_id: DocId,
    },
    /// 请求创建新文档
    CreateDoc {
        name: String,
    },
    /// 重命名文档
    RenameDoc {
        old_path: String,
        new_path: String,
    },
    /// 删除文档
    DeleteDoc {
        path: String,
    },
    /// 复制文档到新位置
    CopyDoc {
        src_path: String,
        dest_path: String,
    },
    /// 移动文档到新位置
    MoveDoc {
        src_path: String,
        dest_path: String,
    },
    /// 调用插件函数
    PluginCall {
        req_id: String,
        plugin_id: String,
        fn_name: String,
        args: Vec<serde_json::Value>,
    },
    /// 全文搜索查询
    Search {
        query: String,
        limit: u32,
    },
    
    // === Manual Merge Messages (手动合并模式) ===
    /// 获取当前同步模式 (Auto/Manual)
    GetSyncMode,
    /// 设置同步模式
    SetSyncMode {
        mode: String, // "auto" or "manual"
    },
    /// 获取待合并操作的数量和预览
    GetPendingOps,
    /// 确认合并所有待处理的操作
    ConfirmMerge,
    /// 丢弃所有待处理的操作
    DiscardPending,
    
    // === Branch Switcher Messages (分支切换) ===
    /// 请求影子库列表 (远程分支)
    ListShadows,
    /// 请求当前分支下的仓库列表 (动态读取 .redb 文件)
    ListRepos,
    /// 切换活动分支
    /// peer_id: None = 本地 (Master), Some = 远程影子库
    SwitchBranch { peer_id: Option<String> },
    
    // === Source Control Messages (版本控制) ===
    /// 获取当前变更列表 (暂存区/未暂存)
    GetChanges,
    /// 暂存指定文件
    StageFile { path: String },
    /// 取消暂存指定文件
    UnstageFile { path: String },
    /// 创建提交
    Commit { message: String },
    /// 获取提交历史
    GetCommitHistory { limit: u32 },
    /// P2P: 将指定 Peer 的分支合入本地
    MergePeer { peer_id: String, doc_id: DocId },
    /// 获取文档的 Diff (用于 Diff 视图)
    GetDocDiff { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    /// 心跳 Pong
    Pong,
    /// 服务端确认操作已持久化
    Ack {
        doc_id: DocId,
        seq: u64,
    },
    /// P2P: 服务端 Hello (响应客户端 Hello)
    SyncHello {
        peer_id: PeerId,
        pub_key: Vec<u8>,
        signature: Vec<u8>,
        vector: VersionVector,
    },
    /// P2P: 服务端向客户端请求数据
    SyncRequest {
        requests: Vec<(PeerId, (u64, u64))>,
    },
    /// P2P: 服务端推送数据给客户端 (批量)
    SyncPush {
        ops: Vec<EncryptedOp>,
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
        version: u64,
    },
    /// 服务端发送完整操作历史 (用于回放)
    History {
        doc_id: DocId,
        ops: Vec<(u64, Op)>,
    },
    /// 服务端发送文档列表
    DocList {
        docs: Vec<(DocId, String)>,
    },
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
    MergeComplete {
        merged_count: u32,
    },
    /// 待合并操作已丢弃
    PendingDiscarded,
    
    // === Branch Switcher Messages (分支切换) ===
    /// 影子库 Peer ID 列表 (远程分支)
    ShadowList {
        shadows: Vec<String>,
    },
    /// 仓库列表 (当前分支下的 .redb 文件)
    RepoList {
        repos: Vec<String>,
    },
    /// 分支切换确认
    BranchSwitched {
        peer_id: Option<String>,
        success: bool,
    },
    /// 编辑请求被拒绝 (Shadow 分支只读)
    EditRejected {
        reason: String,
    },
    
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
    CommitHistory {
        commits: Vec<CommitInfo>,
    },
    /// 文档 Diff 响应 (用于 Diff 视图)
    DocDiff {
        /// 文件路径
        path: String,
        /// 已提交版本内容
        old_content: String,
        /// 当前版本内容
        new_content: String,
    },
    
    /// 错误消息
    Error(String),
}
