// crates\core\src\protocol
//! 服务端 WebSocket 消息协议。
//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 07_network#projection-recovery-contract
//!   - 09_web_thin_client_ledger#web-edit-intent

use super::confirmed_op::ConfirmedOp;
use super::error::ServerError;
use super::merge_conflict::{ConflictHunk, MergeConflictAction};
use crate::models::{DocId, PeerFactSeq, PeerId, RepoId, VersionVector};
use crate::protocol::ProjectionRecoveryRequired;
use crate::protocol::ScopeNonce;
use crate::protocol::SyncPushHeader;
use crate::protocol::SyncSourceProof;
use crate::security::EncryptedOp;
use crate::source_control::diff_projection::DiffProjection;
use crate::source_control::{ChangeEntry, CommitFileDiffSummary, CommitInfo, ExternalApplyReceipt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoListEntry {
    pub repo_id: RepoId,
    pub name: String,
    pub execution_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[rustfmt::skip]
pub enum ServerMessage {
    Pong,
    Ack { repo_id: RepoId, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, doc_id: DocId, seq: u64, client_op_id: u64 },
    SyncHello { peer_id: PeerId, repo_id: RepoId, scope_nonce: ScopeNonce, pub_key: Vec<u8>, signature: Vec<u8>, vector: VersionVector },
    WriteReady { peer_id: PeerId, repo_id: RepoId, scope_nonce: ScopeNonce, #[serde(default)] branch: Option<PeerId> },
    SyncRequest { repo_id: RepoId, #[serde(default)] branch: Option<PeerId>, #[serde(default)] known_vector: VersionVector, requests: Vec<(PeerId, (PeerFactSeq, PeerFactSeq))> },
    SyncSnapshotRequest { #[serde(alias = "peer_id")] source_peer_id: PeerId, repo_id: RepoId, #[serde(default)] known_vector: VersionVector, #[serde(default)] reason: Option<String> },
    SyncPush { source_peer_id: PeerId, repo_id: RepoId, range_start: PeerFactSeq, range_end: PeerFactSeq, header: SyncPushHeader, scope_nonce: ScopeNonce, #[serde(default)] branch: Option<PeerId>, encrypted_payload: Vec<EncryptedOp> },
    SyncPushSnapshot { source_peer_id: PeerId, repo_id: RepoId, waterline: PeerFactSeq, scope_nonce: ScopeNonce, #[serde(default)] branch: Option<PeerId>, #[serde(default)] server_vector: VersionVector, #[serde(default)] snapshot_kind: Option<String>, #[serde(default)] source_proof: Option<SyncSourceProof>, payload: Vec<EncryptedOp> },
    ChatChunk { req_id: String, delta: Option<String>, finish_reason: Option<String> },
    NewOp { repo_id: RepoId, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, doc_id: DocId, entry: ConfirmedOp },
    ProjectionRecoveryRequired(ProjectionRecoveryRequired),
    Snapshot { repo_id: RepoId, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, doc_id: DocId, request_id: u64, content: String, base_seq: u64, version: u64, delta_ops: Vec<ConfirmedOp> },
    History { repo_id: RepoId, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, doc_id: DocId, request_id: u64, ops: Vec<ConfirmedOp> },
    DocList { #[serde(default)] request_id: Option<String>, #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, docs: Vec<(DocId, String)> },
    PluginResponse { req_id: String, #[serde(with = "crate::protocol::json_wire::option")] result: Option<serde_json::Value>, error: Option<ServerError> },
    SearchResults { request_id: String, #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, results: Vec<(String, String, f32)> },
    SyncModeStatus { #[serde(default)] request_id: Option<String>, #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, mode: String },
    PendingOpsInfo { #[serde(default)] request_id: Option<String>, #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, count: u32, previews: Vec<(String, String, String)> },
    MergeComplete { #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, merged_count: u32 },
    PendingDiscarded { #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64> },
    ShadowList { #[serde(default)] request_id: Option<String>, #[serde(default)] scope_nonce: Option<u64>, shadows: Vec<String> },
    RepoList { #[serde(default)] request_id: Option<String>, #[serde(default)] branch: Option<String>, #[serde(default)] scope_nonce: Option<u64>, repos: Vec<String>, #[serde(default)] repo_entries: Vec<RepoListEntry> },
    BranchSwitched { peer_id: Option<String>, success: bool, #[serde(default)] switch_nonce: Option<u64> },
    RepoSwitched { #[serde(default)] branch: Option<String>, name: String, uuid: String, #[serde(default)] switch_nonce: Option<u64> },
    PeerDeleted { peer_id: String, #[serde(default)] scope_nonce: Option<u64> },
    EditRejected { scope_nonce: ScopeNonce, doc_id: DocId, client_op_id: u64, error: ServerError },
    ChangesList { #[serde(default)] request_id: Option<String>, #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, staged: Vec<ChangeEntry>, unstaged: Vec<ChangeEntry>, #[serde(default)] confirmed: Vec<ChangeEntry> },
    ExternalApplyAck { request_id: String, receipt: ExternalApplyReceipt, repo_id: RepoId, #[serde(default)] branch: Option<PeerId>, scope_nonce: ScopeNonce },
    StageAck { #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, path: String },
    UnstageAck { #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, path: String },
    CommitAck { #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, commit_id: String, timestamp: i64 },
    CommitHistory { #[serde(default)] request_id: Option<String>, #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, commits: Vec<CommitInfo> },
    DocDiff { #[serde(default)] request_id: Option<String>, #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, #[serde(default)] doc_id: Option<DocId>, path: String, projection: Arc<DiffProjection> },
    MergeConflict { #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, doc_id: DocId, path: String, projection: Arc<DiffProjection>, result_content: String, actions: Vec<MergeConflictAction>, conflicts: Vec<ConflictHunk> },
    CommitDiffResult { #[serde(default)] request_id: Option<String>, #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, files: Vec<CommitFileDiffSummary> },
    DiffProjectionResult { request_id: String, revision: u64, repo_id: RepoId, #[serde(default)] branch: Option<PeerId>, scope_nonce: ScopeNonce, projection: Arc<DiffProjection> },
    DiffProjectionError { request_id: String, revision: u64, repo_id: RepoId, #[serde(default)] branch: Option<PeerId>, scope_nonce: ScopeNonce, error: ServerError },
    DiscardAck { #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, path: String },
    TreeUpdate { #[serde(default)] request_id: Option<String>, #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, delta: crate::tree::TreeDelta },
    ProtocolError { error: ServerError, #[serde(default)] switch_nonce: Option<u64>, #[serde(default)] scope_nonce: Option<u64> },
    FsChangeDetected { #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, path: String, change_type: String, #[serde(default)] has_conflict: bool },
    ConflictResolved { #[serde(default)] repo_id: Option<RepoId>, #[serde(default)] branch: Option<PeerId>, #[serde(default)] scope_nonce: Option<u64>, path: String, resolution: String },
    KeyProvide { repo_id: RepoId, scope_nonce: ScopeNonce, #[serde(default)] branch: Option<PeerId>, repo_key: Vec<u8> },
    KeyDenied { #[serde(default)] repo_id: Option<RepoId>, scope_nonce: ScopeNonce, #[serde(default)] branch: Option<PeerId>, error: ServerError },
    SystemMetrics { cpu_usage_percent: f32, memory_used_mb: u64, active_connections: u32, ops_processed: u64, uptime_secs: u64, db_size_bytes: u64, doc_count: u32 },
}
