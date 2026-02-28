// crates\core\src\protocol
//! # Client Messages (客户端消息)

use crate::models::{DocId, Op, PeerId, VersionVector};
use crate::security::EncryptedOp;
use serde::{Deserialize, Serialize};

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
    /// 请求快照 (差异过大时)
    SyncSnapshotRequest {
        peer_id: PeerId,
        repo_id: crate::models::RepoId,
    },
    /// 推送加密操作记录给对端 (Envelope Mode)
    SyncPush { ops: Vec<EncryptedOp> },
    /// 推送快照给对端 (Envelope Mode)
    SyncPushSnapshot {
        peer_id: PeerId,
        repo_id: crate::models::RepoId,
        ops: Vec<EncryptedOp>, // Snapshot Ops
    },
    /// 客户端发送编辑操作 (针对特定文档)
    Edit {
        doc_id: DocId,
        op: Op,
        client_id: u64,
    },
    /// 请求文档的完整操作历史
    RequestHistory { doc_id: DocId },
    /// 请求所有已知文档的列表
    ListDocs,
    /// 请求打开指定文档 (获取快照)
    OpenDoc { doc_id: DocId },
    /// 请求创建新文档
    CreateDoc { name: String },
    /// 重命名文档
    RenameDoc { old_path: String, new_path: String },
    /// 删除文档
    DeleteDoc { path: String },
    /// 复制文档到新位置
    CopyDoc { src_path: String, dest_path: String },
    /// 移动文档到新位置
    MoveDoc { src_path: String, dest_path: String },
    /// 调用插件函数
    PluginCall {
        req_id: String,
        plugin_id: String,
        fn_name: String,
        args: Vec<serde_json::Value>,
    },
    /// 全文搜索查询
    Search { query: String, limit: u32 },

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
    /// 切换当前仓库 (.redb 文件)
    /// name: 仓库名称 (e.g. "default.redb" or "knowledge-base")
    SwitchRepo { name: String },
    /// 删除指定 Peer 的远端分支 (物理删除)
    DeletePeer { peer_id: String },

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
    /// 放弃文件变更 (恢复到已提交状态)
    DiscardFile { path: String },

    /// 批量暂存文件
    ///
    /// 注意: 协议枚举必须追加新变体，避免破坏 bincode 兼容性。
    StageFiles { paths: Vec<String> },
    /// 批量取消暂存文件
    ///
    /// 注意: 协议枚举必须追加新变体，避免破坏 bincode 兼容性。
    UnstageFiles { paths: Vec<String> },

    // === E2EE Key Exchange (密钥交换) ===
    /// 请求当前仓库的 RepoKey (通过已认证的 WSS 通道)
    ///
    /// **Pre-condition**: 客户端已通过 JWT 认证。
    /// **Post-condition**: 服务端回复 `ServerMessage::KeyProvide`。
    RequestKey,
}
