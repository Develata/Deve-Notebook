//! plan_ref:
//!   - 03_storage/index#browser-storage-layering
//!   - 04_repository#repo-scope-runtime
//!
use crate::storage::DegradedSyncMode;
use crate::storage::identity::StoredPeerIdentity;
use deve_core::models::{DocId, PeerId, VersionVector};
use leptos::prelude::*;

use super::super::navigation::PendingNavigation;
use crate::runtime::document::pending::PendingLocalEdits;
use crate::runtime::domain::{PendingBranchSwitch, PendingRepoSwitch};

#[derive(Clone, Debug, PartialEq)]
pub struct PeerSession {
    pub id: PeerId,
    pub vector: VersionVector,
    pub last_seen: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub req_id: Option<String>,
    pub ts_ms: u64,
}

#[derive(Clone, Copy)]
pub struct SwitchScopeSignals {
    pub current_doc: ReadSignal<Option<DocId>>,
    pub pending_local_edits: ReadSignal<PendingLocalEdits>,
    pub set_pending_navigation: WriteSignal<Option<PendingNavigation>>,
    pub current_repo: ReadSignal<Option<String>>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchSwitch>>,
    pub pending_repo_switch: ReadSignal<Option<PendingRepoSwitch>>,
    pub set_handshake_ready: WriteSignal<bool>,
    pub set_handshake_scope_nonce: WriteSignal<Option<u64>>,
    pub set_pending_branch_switch: WriteSignal<Option<PendingBranchSwitch>>,
    pub set_pending_repo_switch: WriteSignal<Option<PendingRepoSwitch>>,
}

#[derive(Clone, Copy)]
pub struct HandshakeSignals {
    pub identity: ReadSignal<Option<StoredPeerIdentity>>,
    pub repo_vector: ReadSignal<VersionVector>,
    pub degraded: ReadSignal<Option<DegradedSyncMode>>,
    pub current_repo: ReadSignal<Option<String>>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchSwitch>>,
    pub set_pending_branch_switch: WriteSignal<Option<PendingBranchSwitch>>,
    pub pending_repo_switch: ReadSignal<Option<PendingRepoSwitch>>,
    pub set_pending_repo_switch: WriteSignal<Option<PendingRepoSwitch>>,
    pub handshake_scope_nonce: ReadSignal<Option<u64>>,
    pub set_handshake_scope_nonce: WriteSignal<Option<u64>>,
    pub handshake_retry_nonce: ReadSignal<u64>,
    pub set_repo_list_request_id: WriteSignal<Option<String>>,
    pub set_doc_list_request_id: WriteSignal<Option<String>>,
    pub set_tree_request_id: WriteSignal<Option<String>>,
    pub set_handshake_ready: WriteSignal<bool>,
}

#[derive(Clone, Copy)]
pub struct RepoSwitchSignals {
    pub current_repo: ReadSignal<Option<String>>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub pending_repo_switch: ReadSignal<Option<PendingRepoSwitch>>,
    pub set_pending_repo_switch: WriteSignal<Option<PendingRepoSwitch>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub set_current_scope_nonce: WriteSignal<u64>,
    pub set_current_repo: WriteSignal<Option<String>>,
    pub set_current_repo_id: WriteSignal<Option<String>>,
    pub set_current_doc: WriteSignal<Option<DocId>>,
}
