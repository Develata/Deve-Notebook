// crates\core\src\protocol
//! 客户端 WebSocket 消息协议。
//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent

use crate::models::{DocId, Op, PeerId, VersionVector};
use crate::protocol::ScPathTarget;
use crate::protocol::ScopeNonce;
use crate::protocol::SessionProof;
use crate::protocol::SyncPushHeader;
use crate::protocol::SyncSourceProof;
use crate::security::EncryptedOp;
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
        #[serde(default)]
        known_vector: VersionVector,
        requests: Vec<(PeerId, (u64, u64))>,
    },
    SyncSnapshotRequest {
        #[serde(alias = "peer_id")]
        source_peer_id: PeerId,
        repo_id: crate::models::RepoId,
        #[serde(default)]
        known_vector: VersionVector,
        #[serde(default)]
        reason: Option<String>,
    },
    SyncPush {
        #[serde(alias = "peer_id")]
        source_peer_id: PeerId,
        repo_id: crate::models::RepoId,
        header: SyncPushHeader,
        #[serde(alias = "ops")]
        encrypted_payload: Vec<EncryptedOp>,
    },
    SyncPushSnapshot {
        #[serde(alias = "peer_id")]
        source_peer_id: PeerId,
        repo_id: crate::models::RepoId,
        #[serde(default)]
        server_vector: VersionVector,
        #[serde(default)]
        snapshot_kind: Option<String>,
        #[serde(default)]
        source_proof: Option<SyncSourceProof>,
        #[serde(alias = "ops", alias = "encrypted_payload")]
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
    CreateDoc {
        name: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
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
    SwitchRepo {
        name: String,
        #[serde(default)]
        switch_nonce: Option<u64>,
    },
    SwitchRepoExact {
        name: String,
        repo_id: crate::models::RepoId,
        #[serde(default)]
        switch_nonce: Option<u64>,
    },
    CreateRepo {
        name: String,
        #[serde(default)]
        switch_nonce: Option<u64>,
    },
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
    CommitAndPush {
        message: String,
        #[serde(default)]
        scope_nonce: Option<u64>,
    },
}
