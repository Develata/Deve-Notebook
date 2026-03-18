// crates\core\src\protocol
//! 服务端 WebSocket 消息协议。

use super::confirmed_op::ConfirmedOp;
use super::error::ServerError;
use crate::models::{DocId, PeerId, RepoId, VersionVector};
use crate::security::EncryptedOp;
use crate::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[rustfmt::skip]
pub enum ServerMessage {
    Pong,
    Ack { repo_id: RepoId, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, doc_id: DocId, seq: u64, client_op_id: u64 },
    SyncHello { peer_id: PeerId, repo_id: RepoId, scope_nonce: u64, pub_key: Vec<u8>, signature: Vec<u8>, vector: VersionVector },
    WriteReady { peer_id: PeerId, repo_id: RepoId, scope_nonce: u64, #[serde(default)] branch: Option<PeerId> },
    SyncRequest { repo_id: RepoId, #[serde(default)] branch: Option<PeerId>, requests: Vec<(PeerId, (u64, u64))> },
    SyncSnapshotRequest { peer_id: PeerId, repo_id: RepoId },
    SyncPush { repo_id: RepoId, scope_nonce: u64, #[serde(default)] branch: Option<PeerId>, ops: Vec<EncryptedOp> },
    SyncPushSnapshot { peer_id: PeerId, repo_id: RepoId, scope_nonce: u64, #[serde(default)] branch: Option<PeerId>, ops: Vec<EncryptedOp> },
    ChatChunk { req_id: String, delta: Option<String>, finish_reason: Option<String> },
    NewOp { repo_id: RepoId, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, doc_id: DocId, entry: ConfirmedOp },
    Snapshot { repo_id: RepoId, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, doc_id: DocId, request_id: u64, content: String, base_seq: u64, version: u64, delta_ops: Vec<ConfirmedOp> },
    History { repo_id: RepoId, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, doc_id: DocId, request_id: u64, ops: Vec<ConfirmedOp> },
    DocList { #[serde(default)] request_id: Option<String>, #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, docs: Vec<(DocId, String)> },
    PluginResponse { req_id: String, result: Option<serde_json::Value>, error: Option<ServerError> },
    SearchResults { request_id: String, #[serde(default)] scope_nonce: Option<u64>, results: Vec<(String, String, f32)> },
    SyncModeStatus { #[serde(default)] request_id: Option<String>, #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, mode: String },
    PendingOpsInfo { #[serde(default)] request_id: Option<String>, #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, count: u32, previews: Vec<(String, String, String)> },
    MergeComplete { #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, merged_count: u32 },
    PendingDiscarded { #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64> },
    ShadowList { #[serde(default)] request_id: Option<String>, #[serde(default)] scope_nonce: Option<u64>, shadows: Vec<String> },
    RepoList { #[serde(default)] request_id: Option<String>, #[serde(default)] branch: Option<String>, #[serde(default)] scope_nonce: Option<u64>, repos: Vec<String> },
    BranchSwitched { peer_id: Option<String>, success: bool, #[serde(default)] switch_nonce: Option<u64> },
    RepoSwitched { #[serde(default)] branch: Option<String>, name: String, uuid: String, #[serde(default)] switch_nonce: Option<u64> },
    PeerDeleted { peer_id: String },
    EditRejected { #[serde(default)] scope_nonce: Option<u64>, error: ServerError },
    ChangesList { #[serde(default)] request_id: Option<String>, #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, staged: Vec<ChangeEntry>, unstaged: Vec<ChangeEntry> },
    StageAck { #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, path: String },
    UnstageAck { #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, path: String },
    CommitAck { #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, commit_id: String, timestamp: i64 },
    CommitHistory { #[serde(default)] request_id: Option<String>, #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, commits: Vec<CommitInfo> },
    DocDiff { #[serde(default)] request_id: Option<String>, #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, path: String, old_content: String, new_content: String },
    CommitDiffResult { #[serde(default)] request_id: Option<String>, #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, diffs: Vec<CommitFileDiff> },
    DiscardAck { #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, path: String },
    TreeUpdate { #[serde(default)] request_id: Option<String>, #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, delta: crate::tree::TreeDelta },
    ProtocolError { error: ServerError, #[serde(default)] switch_nonce: Option<u64>, #[serde(default)] scope_nonce: Option<u64> },
    FsChangeDetected { #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, path: String, change_type: String, #[serde(default)] has_conflict: bool },
    ConflictResolved { #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, path: String, resolution: String },
    KeyProvide { repo_id: RepoId, scope_nonce: u64, #[serde(default)] branch: Option<PeerId>, repo_key: Vec<u8> },
    KeyDenied { #[serde(default)] repo_id: Option<RepoId>, scope_nonce: u64, #[serde(default)] branch: Option<PeerId>, error: ServerError },
    SystemMetrics { cpu_usage_percent: f32, memory_used_mb: u64, active_connections: u32, ops_processed: u64, uptime_secs: u64, db_size_bytes: u64, doc_count: u32 },
}
