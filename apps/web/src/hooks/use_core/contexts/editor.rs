//! plan_ref:
//!   - 10_rendering#large-document-runtime
//!   - 10_rendering#document-authority-bridge
//!
use crate::editor::EditorStats;
use deve_core::models::{DocId, PeerId};
use leptos::prelude::*;

use super::super::navigation::PendingNavigation;
use super::super::types::PendingBranchTarget;
use crate::runtime::document::pending::PendingLocalEdits;

#[derive(Clone)]
pub struct EditorContext {
    pub docs: ReadSignal<Vec<(DocId, String)>>,
    pub current_doc: ReadSignal<Option<DocId>>,
    pub stats: ReadSignal<EditorStats>,
    pub on_stats: Callback<EditorStats>,
    pub load_state: ReadSignal<String>,
    pub set_load_state: WriteSignal<String>,
    pub load_progress: ReadSignal<(usize, usize)>,
    pub set_load_progress: WriteSignal<(usize, usize)>,
    pub load_eta_ms: ReadSignal<u64>,
    pub set_load_eta_ms: WriteSignal<u64>,
    pub doc_version: ReadSignal<u64>,
    pub set_doc_version: WriteSignal<u64>,
    pub playback_version: ReadSignal<u64>,
    pub set_playback_version: WriteSignal<u64>,
    pub is_spectator: Signal<bool>,
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
    pub handshake_ready: ReadSignal<bool>,
    pub handshake_scope_nonce: ReadSignal<Option<u64>>,
    pub pending_local_edits: ReadSignal<PendingLocalEdits>,
    pub set_pending_local_edits: WriteSignal<PendingLocalEdits>,
    pub set_pending_navigation: WriteSignal<Option<PendingNavigation>>,
}
