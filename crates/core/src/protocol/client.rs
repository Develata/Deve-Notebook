// crates\core\src\protocol
//! 客户端 WebSocket 消息协议。

use crate::models::{DocId, Op, PeerId, VersionVector};
use crate::protocol::ScPathTarget;
use crate::security::EncryptedOp;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Ping,
    SyncHello {
        peer_id: PeerId,
        pub_key: Vec<u8>,
        signature: Vec<u8>,
        vector: VersionVector,
        repo_id: crate::models::RepoId,
    },
    RegisterWriter {
        peer_id: PeerId,
        repo_id: crate::models::RepoId,
    },
    SyncRequest {
        repo_id: crate::models::RepoId,
        requests: Vec<(PeerId, (u64, u64))>,
    },
    SyncSnapshotRequest {
        peer_id: PeerId,
        repo_id: crate::models::RepoId,
    },
    SyncPush {
        repo_id: crate::models::RepoId,
        ops: Vec<EncryptedOp>,
    },
    SyncPushSnapshot {
        peer_id: PeerId,
        repo_id: crate::models::RepoId,
        ops: Vec<EncryptedOp>,
    },
    Edit {
        doc_id: DocId,
        op: Op,
        client_id: u64,
        client_op_id: u64,
    },
    RequestHistory {
        doc_id: DocId,
        request_id: u64,
    },
    ListDocs,
    OpenDoc {
        doc_id: DocId,
        request_id: u64,
    },
    CreateDoc {
        name: String,
    },
    RenameDoc {
        old_path: String,
        new_path: String,
    },
    DeleteDoc {
        path: String,
    },
    CopyDoc {
        src_path: String,
        dest_path: String,
    },
    MoveDoc {
        src_path: String,
        dest_path: String,
    },
    PluginCall {
        req_id: String,
        plugin_id: String,
        fn_name: String,
        args: Vec<serde_json::Value>,
    },
    Search {
        request_id: String,
        query: String,
        limit: u32,
    },
    GetSyncMode,
    SetSyncMode {
        mode: String,
    },
    GetPendingOps,
    ConfirmMerge,
    DiscardPending,
    ListShadows {
        request_id: String,
    },
    ListRepos {
        request_id: String,
    },
    SwitchBranch {
        peer_id: Option<String>,
    },
    SwitchRepo {
        name: String,
    },
    DeletePeer {
        peer_id: String,
    },
    GetChanges,
    StageFile {
        target: ScPathTarget,
    },
    UnstageFile {
        target: ScPathTarget,
    },
    Commit {
        message: String,
    },
    GetCommitHistory {
        limit: u32,
    },
    MergePeer {
        peer_id: String,
        doc_id: DocId,
    },
    GetDocDiff {
        target: ScPathTarget,
    },
    DiscardFile {
        target: ScPathTarget,
    },
    StageFiles {
        targets: Vec<ScPathTarget>,
    },
    UnstageFiles {
        targets: Vec<ScPathTarget>,
    },
    GetCommitDiff {
        commit_a: Option<String>,
        commit_b: String,
    },
    RequestKey,
    ResolveConflict {
        target: ScPathTarget,
        resolution: crate::source_control::ConflictResolution,
    },
    CommitAndPush {
        message: String,
    },
}
