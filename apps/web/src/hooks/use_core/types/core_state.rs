//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::editor::EditorStats;
use crate::runtime::CoreRuntimeClients;
use crate::runtime::domain::{
    AiBackendMode, ChatMessage, LoadPhase, PeerSession, PendingBranchSwitch, PendingOpsPreview,
    PendingRepoSwitch, RepoRemoveRequest, RepoRenameRequest, RepoSwitchRequest, SearchHit,
    SyncModeState,
};
use deve_core::models::{DocId, PeerId};
use deve_core::protocol::RepoListEntry;
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo, ConflictResolution};
use deve_core::tree::FileNode;
use leptos::prelude::*;
use std::collections::HashMap;

use super::super::navigation::PendingNavigation;
use super::super::source_control_notice::SourceControlNotice;
use super::super::state::PluginResponse;
use crate::runtime::document::pending::PendingLocalEdits;
use crate::runtime::source_control_client::diff_session::DiffSessionWire;

#[derive(Clone)]
pub struct CoreState {
    pub ws: WsService,
    pub runtime_clients: CoreRuntimeClients,
    pub docs: ReadSignal<Vec<(DocId, String)>>,
    pub current_doc: ReadSignal<Option<DocId>>,
    pub set_current_doc: WriteSignal<Option<DocId>>,
    pub status_text: Signal<String>,
    pub sync_banner: Signal<Option<String>>,
    pub set_sync_banner: WriteSignal<Option<String>>,
    pub stats: ReadSignal<EditorStats>,
    pub peers: ReadSignal<HashMap<PeerId, PeerSession>>,
    pub handshake_ready: ReadSignal<bool>,
    pub handshake_scope_nonce: ReadSignal<Option<u64>>,
    pub on_retry_peer_registration: Callback<()>,
    pub pending_local_edits: ReadSignal<PendingLocalEdits>,
    pub set_pending_local_edits: WriteSignal<PendingLocalEdits>,
    pub pending_navigation: ReadSignal<Option<PendingNavigation>>,
    pub set_pending_navigation: WriteSignal<Option<PendingNavigation>>,
    pub on_doc_select: Callback<DocId>,
    pub on_doc_create: Callback<String>,
    pub on_doc_rename: Callback<(String, String)>,
    pub on_doc_delete: Callback<String>,
    pub on_doc_copy: Callback<(String, String)>,
    pub on_doc_move: Callback<(String, String)>,
    pub on_stats: Callback<EditorStats>,
    pub plugin_last_response: ReadSignal<PluginResponse>,
    pub plugin_request_ids: ReadSignal<Vec<String>>,
    pub on_plugin_call: Callback<(String, String, String, Vec<serde_json::Value>)>,
    pub chat_messages: ReadSignal<Vec<ChatMessage>>,
    pub set_chat_messages: WriteSignal<Vec<ChatMessage>>,
    pub is_chat_streaming: ReadSignal<bool>,
    pub set_is_chat_streaming: WriteSignal<bool>,
    pub ai_mode: ReadSignal<AiBackendMode>,
    pub set_ai_mode: WriteSignal<AiBackendMode>,
    pub search_results: ReadSignal<Vec<SearchHit>>,
    pub on_search: Callback<String>,
    pub load_state: ReadSignal<LoadPhase>,
    pub set_load_state: WriteSignal<LoadPhase>,
    pub load_progress: ReadSignal<(usize, usize)>,
    pub set_load_progress: WriteSignal<(usize, usize)>,
    pub load_eta_ms: ReadSignal<u64>,
    pub set_load_eta_ms: WriteSignal<u64>,
    pub sync_mode: ReadSignal<SyncModeState>,
    pub pending_ops_count: ReadSignal<u32>,
    pub pending_ops_previews: ReadSignal<Vec<PendingOpsPreview>>,
    pub on_get_sync_mode: Callback<()>,
    pub on_set_sync_mode: Callback<String>,
    pub on_get_pending_ops: Callback<()>,
    pub on_confirm_merge: Callback<()>,
    pub on_discard_pending: Callback<()>,
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub set_active_branch: WriteSignal<Option<PeerId>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchSwitch>>,
    pub on_switch_branch: Callback<Option<String>>,
    pub current_repo: ReadSignal<Option<String>>,
    pub set_current_repo: WriteSignal<Option<String>>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub set_current_repo_id: WriteSignal<Option<String>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub pending_repo_switch: ReadSignal<Option<PendingRepoSwitch>>,
    pub on_switch_repo: Callback<RepoSwitchRequest>,
    pub on_create_repo: Callback<String>,
    pub on_rename_repo: Callback<RepoRenameRequest>,
    pub on_remove_repo: Callback<RepoRemoveRequest>,
    pub shadow_repos: ReadSignal<Vec<String>>,
    pub on_list_shadows: Callback<()>,
    pub repo_list: ReadSignal<Vec<String>>,
    pub repo_entries: ReadSignal<Vec<RepoListEntry>>,
    pub doc_version: ReadSignal<u64>,
    pub set_doc_version: WriteSignal<u64>,
    pub playback_version: ReadSignal<u64>,
    pub set_playback_version: WriteSignal<u64>,
    pub is_spectator: Signal<bool>,
    pub staged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub unstaged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub confirmed_changes: ReadSignal<Vec<ChangeEntry>>,
    pub commit_history: ReadSignal<Vec<CommitInfo>>,
    pub commit_history_request_id: ReadSignal<Option<String>>,
    pub commit_diff_request_id: ReadSignal<Option<String>>,
    pub set_commit_diff_request_id: WriteSignal<Option<String>>,
    pub on_get_changes: Callback<()>,
    pub on_stage_file: Callback<ChangeEntry>,
    pub on_stage_files: Callback<Vec<ChangeEntry>>,
    pub on_unstage_file: Callback<ChangeEntry>,
    pub on_unstage_files: Callback<Vec<ChangeEntry>>,
    pub on_discard_file: Callback<ChangeEntry>,
    pub on_commit: Callback<String>,
    pub on_get_history: Callback<u32>,
    pub diff_content: ReadSignal<Option<DiffSessionWire>>,
    pub set_diff_content: WriteSignal<Option<DiffSessionWire>>,
    pub on_get_doc_diff: Callback<ChangeEntry>,
    pub commit_diff_result: ReadSignal<Vec<CommitFileDiff>>,
    pub set_commit_diff_result: WriteSignal<Vec<CommitFileDiff>>,
    pub source_control_notice: ReadSignal<Option<SourceControlNotice>>,
    pub set_source_control_notice: WriteSignal<Option<SourceControlNotice>>,
    pub on_resolve_conflict: Callback<(ChangeEntry, ConflictResolution)>,
    pub on_get_commit_diff: Callback<(Option<String>, String)>,
    pub on_commit_and_push: Callback<String>,
    pub on_merge_peer: Callback<String>,
    pub tree_nodes: ReadSignal<Vec<FileNode>>,
    pub set_explicit_home: WriteSignal<bool>,
}
