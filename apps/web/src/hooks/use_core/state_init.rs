//! `CoreSignals` 初始化工厂。

use crate::api::ConnectionStatus;
use crate::editor::EditorStats;
use crate::storage::DegradedSyncMode;
use deve_core::models::{DocId, PeerId};
use deve_core::source_control::CommitFileDiff;
use deve_core::tree::FileNode;
use leptos::prelude::*;
use std::collections::HashMap;

use super::contexts::SystemMetricsData;
use super::diff_session::DiffSessionWire;
use super::pending::PendingLocalEdits;
use super::state::{CoreSignals, PluginResponse};
use super::types::PeerSession;

/// 初始化所有核心信号。
///
/// Invariant:
/// - `is_spectator` 在远端分支、降级存储或断连时必须为真。
pub fn init_signals(connection_status: ReadSignal<ConnectionStatus>) -> CoreSignals {
    let (docs, set_docs) = signal(Vec::<(DocId, String)>::new());
    let (current_doc, set_current_doc) = signal(None::<DocId>);
    let (stats, set_stats) = signal(EditorStats::default());
    let (peers, set_peers) = signal(HashMap::<PeerId, PeerSession>::new());
    let (handshake_ready, set_handshake_ready) = signal(false);
    let (pending_local_edits, set_pending_local_edits) = signal(PendingLocalEdits::new());
    let (plugin_response, set_plugin_response) = signal(PluginResponse::default());
    let (chat_messages, set_chat_messages) = signal(Vec::new());
    let (is_chat_streaming, set_is_chat_streaming) = signal(false);
    let (ai_mode, set_ai_mode) = signal("agent-bridge".to_string());
    let (search_results, set_search_results) = signal(Vec::new());
    let (load_state, set_load_state) = signal("ready".to_string());
    let (load_progress, set_load_progress) = signal((0usize, 0usize));
    let (load_eta_ms, set_load_eta_ms) = signal(0u64);
    let (sync_mode, set_sync_mode) = signal("auto".to_string());
    let (pending_ops_count, set_pending_ops_count) = signal(0u32);
    let (pending_ops_previews, set_pending_ops_previews) = signal(Vec::new());
    let (active_branch, set_active_branch) = signal(None::<PeerId>);
    let (current_repo, set_current_repo) = signal(None::<String>);
    let (current_repo_id, set_current_repo_id) = signal(None::<String>);
    let (shadow_repos, set_shadow_repos) = signal(Vec::new());
    let (repo_list, set_repo_list) = signal(Vec::new());
    let (doc_version, set_doc_version) = signal(0u64);
    let (playback_version, set_playback_version) = signal(0u64);
    let (degraded_sync_mode, set_degraded_sync_mode) = signal(None::<DegradedSyncMode>);
    let (sync_banner, set_sync_banner) = signal(None::<String>);
    let is_spectator = Memo::new(move |_| {
        let disconnected = connection_status.get() != ConnectionStatus::Connected;
        active_branch.get().is_some() || degraded_sync_mode.get().is_some() || disconnected
    });
    let (staged_changes, set_staged_changes) = signal(Vec::new());
    let (unstaged_changes, set_unstaged_changes) = signal(Vec::new());
    let (commit_history, set_commit_history) = signal(Vec::new());
    let (diff_content, set_diff_content) = signal(None::<DiffSessionWire>);
    let (commit_diff_result, set_commit_diff_result) = signal(Vec::<CommitFileDiff>::new());
    let (tree_nodes, set_tree_nodes) = signal(Vec::<FileNode>::new());
    let (system_metrics, set_system_metrics) = signal(None::<SystemMetricsData>);
    let (_, set_explicit_home) = signal(false);

    CoreSignals {
        docs,
        set_docs,
        current_doc,
        set_current_doc,
        stats,
        set_stats,
        peers,
        set_peers,
        handshake_ready,
        set_handshake_ready,
        pending_local_edits,
        set_pending_local_edits,
        plugin_response,
        set_plugin_response,
        chat_messages,
        set_chat_messages,
        is_chat_streaming,
        set_is_chat_streaming,
        ai_mode,
        set_ai_mode,
        search_results,
        set_search_results,
        load_state,
        set_load_state,
        load_progress,
        set_load_progress,
        load_eta_ms,
        set_load_eta_ms,
        sync_mode,
        set_sync_mode,
        pending_ops_count,
        set_pending_ops_count,
        pending_ops_previews,
        set_pending_ops_previews,
        active_branch,
        set_active_branch,
        current_repo,
        set_current_repo,
        current_repo_id,
        set_current_repo_id,
        shadow_repos,
        set_shadow_repos,
        repo_list,
        set_repo_list,
        doc_version,
        set_doc_version,
        playback_version,
        set_playback_version,
        is_spectator,
        staged_changes,
        set_staged_changes,
        unstaged_changes,
        set_unstaged_changes,
        commit_history,
        set_commit_history,
        diff_content,
        set_diff_content,
        commit_diff_result,
        set_commit_diff_result,
        tree_nodes,
        set_tree_nodes,
        system_metrics,
        set_system_metrics,
        degraded_sync_mode,
        set_degraded_sync_mode,
        sync_banner,
        set_sync_banner,
        set_explicit_home,
    }
}
