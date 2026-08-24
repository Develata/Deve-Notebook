// crates\core\src\protocol
//! 客户端 WebSocket 消息协议。
//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 07_network#projection-recovery-contract
//!   - 09_web_thin_client_ledger#web-edit-intent
//!   - 09_web_thin_client_ledger#document-create-intent

use crate::models::{DocId, Op, PeerFactSeq, PeerId, VersionVector};
use crate::protocol::ScPathTarget;
use crate::protocol::ScopeNonce;
use crate::protocol::SessionProof;
use crate::protocol::SyncPushHeader;
use crate::protocol::SyncSourceProof;
use crate::protocol::{
    DocumentCreateRequest, RemoteImportRequest, RemoteProjectionPushRequest, RepoControlRequest,
};
use crate::security::EncryptedOp;
use crate::source_control::CommitFileDiffTarget;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Ping,
    SyncHello {
        peer_id: PeerId,
        peer_pubkey: Vec<u8>,
        session_proof: SessionProof,
        vector: VersionVector,
        repo_id: crate::models::RepoId,
        scope_nonce: ScopeNonce,
    },
    RegisterWriter {
        peer_id: PeerId,
        repo_id: crate::models::RepoId,
        scope_nonce: ScopeNonce,
    },
    SyncRequest {
        repo_id: crate::models::RepoId,
        known_vector: VersionVector,
        requests: Vec<(PeerId, (PeerFactSeq, PeerFactSeq))>,
    },
    SyncSnapshotRequest {
        source_peer_id: PeerId,
        repo_id: crate::models::RepoId,
        known_vector: VersionVector,
        #[serde(default)]
        reason: Option<String>,
    },
    SyncPush {
        source_peer_id: PeerId,
        repo_id: crate::models::RepoId,
        range_start: PeerFactSeq,
        range_end: PeerFactSeq,
        header: SyncPushHeader,
        encrypted_payload: Vec<EncryptedOp>,
    },
    SyncPushSnapshot {
        source_peer_id: PeerId,
        repo_id: crate::models::RepoId,
        waterline: PeerFactSeq,
        server_vector: VersionVector,
        #[serde(default)]
        snapshot_kind: Option<String>,
        #[serde(default)]
        source_proof: Option<SyncSourceProof>,
        payload: Vec<EncryptedOp>,
    },
    Edit {
        doc_id: DocId,
        op: Op,
        client_id: u64,
        client_op_id: u64,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    RequestHistory {
        doc_id: DocId,
        request_id: u64,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    ListDocs {
        request_id: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    OpenDoc {
        doc_id: DocId,
        request_id: u64,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    DocumentCreate(DocumentCreateRequest),
    RenameDoc {
        old_path: String,
        new_path: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    DeleteDoc {
        path: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    CopyDoc {
        src_path: String,
        dest_path: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    MoveDoc {
        src_path: String,
        dest_path: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    PluginCall {
        req_id: String,
        plugin_id: String,
        fn_name: String,
        #[serde(with = "crate::protocol::json_wire::vec")]
        args: Vec<serde_json::Value>,
    },
    Search {
        request_id: String,
        query: String,
        limit: u32,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    GetSyncMode {
        request_id: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    SetSyncMode {
        mode: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    GetPendingOps {
        request_id: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    ConfirmMerge {
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    ResolveMergeConflict {
        doc_id: DocId,
        action: super::merge_conflict::MergeConflictAction,
        #[serde(default)]
        result_content: Option<String>,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    DiscardPending {
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    ListShadows {
        request_id: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    ListRepos {
        request_id: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    SwitchBranch {
        peer_id: Option<String>,
        #[serde(default)]
        switch_nonce: Option<u64>,
    },
    SwitchRepoExact {
        repo_id: crate::models::RepoId,
        #[serde(default)]
        switch_nonce: Option<u64>,
    },
    RepoControl(RepoControlRequest),
    DeletePeer {
        peer_id: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    GetChanges {
        request_id: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    StageFile {
        target: ScPathTarget,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    UnstageFile {
        target: ScPathTarget,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    Commit {
        message: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    ApplyExternalChanges {
        request_id: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    GetCommitHistory {
        request_id: String,
        limit: u32,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    MergePeer {
        peer_id: String,
        doc_id: DocId,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    GetDocDiff {
        request_id: String,
        target: ScPathTarget,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    DiscardFile {
        target: ScPathTarget,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    StageFiles {
        targets: Vec<ScPathTarget>,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    UnstageFiles {
        targets: Vec<ScPathTarget>,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    GetCommitDiff {
        request_id: String,
        commit_a: Option<String>,
        commit_b: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    GetCommitFileDiff {
        request_id: String,
        commit_a: Option<String>,
        commit_b: String,
        target: CommitFileDiffTarget,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    ComputeDiffProjection {
        request_id: String,
        revision: u64,
        base_content: String,
        target_content: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    RequestKey {
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    ResolveConflict {
        target: ScPathTarget,
        resolution: crate::source_control::ConflictResolution,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
    RemoteImport(RemoteImportRequest),
    RemoteProjectionPush(RemoteProjectionPushRequest),
}
