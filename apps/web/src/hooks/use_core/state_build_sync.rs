//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use deve_core::models::PeerId;
use leptos::prelude::*;
use std::collections::HashMap;

use super::super::callbacks::{SwitchCallbacks, SyncCallbacks};
use super::super::state::CoreSignals;
use super::super::types::{PeerSession, PendingBranchTarget};

pub(super) struct SyncStateSection {
    pub peers: ReadSignal<HashMap<PeerId, PeerSession>>,
    pub handshake_ready: ReadSignal<bool>,
    pub handshake_scope_nonce: ReadSignal<Option<u64>>,
    pub sync_mode: ReadSignal<String>,
    pub pending_ops_count: ReadSignal<u32>,
    pub pending_ops_previews: ReadSignal<Vec<(String, String, String)>>,
    pub on_get_sync_mode: Callback<()>,
    pub on_set_sync_mode: Callback<String>,
    pub on_get_pending_ops: Callback<()>,
    pub on_confirm_merge: Callback<()>,
    pub on_discard_pending: Callback<()>,
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub set_active_branch: WriteSignal<Option<PeerId>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub current_repo: ReadSignal<Option<String>>,
    pub set_current_repo: WriteSignal<Option<String>>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub set_current_repo_id: WriteSignal<Option<String>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
    pub shadow_repos: ReadSignal<Vec<String>>,
    pub on_list_shadows: Callback<()>,
    pub repo_list: ReadSignal<Vec<String>>,
    pub doc_version: ReadSignal<u64>,
    pub set_doc_version: WriteSignal<u64>,
    pub playback_version: ReadSignal<u64>,
    pub set_playback_version: WriteSignal<u64>,
    pub is_spectator: Signal<bool>,
    pub on_merge_peer: Callback<String>,
}

pub(super) fn build_sync_section(
    signals: &CoreSignals,
    sync: &SyncCallbacks,
    _switch: &SwitchCallbacks,
) -> SyncStateSection {
    SyncStateSection {
        peers: signals.peers,
        handshake_ready: signals.handshake_ready,
        handshake_scope_nonce: signals.handshake_scope_nonce,
        sync_mode: signals.sync_mode,
        pending_ops_count: signals.pending_ops_count,
        pending_ops_previews: signals.pending_ops_previews,
        on_get_sync_mode: sync.on_get_sync_mode,
        on_set_sync_mode: sync.on_set_sync_mode,
        on_get_pending_ops: sync.on_get_pending_ops,
        on_confirm_merge: sync.on_confirm_merge,
        on_discard_pending: sync.on_discard_pending,
        active_branch: signals.active_branch,
        set_active_branch: signals.set_active_branch,
        pending_branch_switch: signals.pending_branch_switch,
        current_repo: signals.current_repo,
        set_current_repo: signals.set_current_repo,
        current_repo_id: signals.current_repo_id,
        set_current_repo_id: signals.set_current_repo_id,
        current_scope_nonce: signals.current_scope_nonce,
        pending_repo_switch: signals.pending_repo_switch,
        shadow_repos: signals.shadow_repos,
        on_list_shadows: sync.on_list_shadows,
        repo_list: signals.repo_list,
        doc_version: signals.doc_version,
        set_doc_version: signals.set_doc_version,
        playback_version: signals.playback_version,
        set_playback_version: signals.set_playback_version,
        is_spectator: signals.is_spectator.into(),
        on_merge_peer: sync.on_merge_peer,
    }
}
