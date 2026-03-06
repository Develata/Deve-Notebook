// apps/web/src/hooks/use_core/mod.rs
//! # Core State Hook (核心状态钩子)
//!
//! 管理前端全局核心状态，并把 WebLightPeer 的浏览器存储分层接入 UI。

pub mod apply;
pub mod callbacks;
pub mod callbacks_sc;
pub mod contexts;
pub mod diff_session;
pub mod effects;
pub mod effects_msg;
pub mod effects_sc;
mod provide;
pub mod state;
mod storage_runtime;
pub mod types;

pub use contexts::*;
pub use types::*;

use crate::api::WsService;
use leptos::prelude::*;

use self::storage_runtime::init_storage_runtime;

/// 初始化核心状态钩子。
pub fn use_core() -> CoreState {
    let ws = WsService::new();
    provide_context(ws.clone());

    let signals = state::init_signals(ws.status);
    let status_signal_for_text = ws.status;
    let degraded_for_text = signals.degraded_sync_mode;
    let status_text = Signal::derive(move || {
        let base = format!("{}", status_signal_for_text.get());
        if degraded_for_text.get().is_some() {
            format!("{base} · Read-only")
        } else {
            base
        }
    });

    // 浏览器 peer identity 现在必须经由 storage_runtime 间接初始化：
    // `localStorage` 只允许承载 UI 偏好，而 repo-scoped identity 需要走
    // `WebCrypto + IndexedDB`，这样才能满足 T3 定义的存储分层与降级语义。
    let (identity, repo_vector) = init_storage_runtime(&signals);

    effects::setup_handshake_effect(&ws, identity, repo_vector, signals.degraded_sync_mode);
    effects::setup_message_effect(&ws, &signals);

    let doc_callbacks = callbacks::create_doc_callbacks(&ws, signals.set_current_doc, signals.set_explicit_home);
    let sync_callbacks = callbacks::create_sync_callbacks(&ws, signals.current_doc);
    let sc_callbacks = callbacks::create_source_control_callbacks(&ws);
    let misc_callbacks =
        callbacks::create_misc_callbacks(&ws, signals.set_stats, signals.load_state);
    let switch_callbacks = callbacks::create_switch_callbacks(&ws);

    let state = CoreState {
        ws,
        docs: signals.docs,
        current_doc: signals.current_doc,
        set_current_doc: signals.set_current_doc,
        status_text,
        sync_banner: signals.sync_banner.into(),
        stats: signals.stats,
        peers: signals.peers,
        on_doc_select: doc_callbacks.on_doc_select,
        on_doc_create: doc_callbacks.on_doc_create,
        on_doc_rename: doc_callbacks.on_doc_rename,
        on_doc_delete: doc_callbacks.on_doc_delete,
        on_doc_copy: doc_callbacks.on_doc_copy,
        on_doc_move: doc_callbacks.on_doc_move,
        on_stats: misc_callbacks.on_stats,
        plugin_last_response: signals.plugin_response,
        on_plugin_call: misc_callbacks.on_plugin_call,
        search_results: signals.search_results,
        on_search: misc_callbacks.on_search,
        load_state: signals.load_state,
        set_load_state: signals.set_load_state,
        load_progress: signals.load_progress,
        set_load_progress: signals.set_load_progress,
        load_eta_ms: signals.load_eta_ms,
        set_load_eta_ms: signals.set_load_eta_ms,
        sync_mode: signals.sync_mode,
        pending_ops_count: signals.pending_ops_count,
        pending_ops_previews: signals.pending_ops_previews,
        on_get_sync_mode: sync_callbacks.on_get_sync_mode,
        on_set_sync_mode: sync_callbacks.on_set_sync_mode,
        on_get_pending_ops: sync_callbacks.on_get_pending_ops,
        on_confirm_merge: sync_callbacks.on_confirm_merge,
        on_discard_pending: sync_callbacks.on_discard_pending,
        active_branch: signals.active_branch,
        set_active_branch: signals.set_active_branch,
        on_switch_branch: switch_callbacks.on_switch_branch,
        current_repo: signals.current_repo,
        set_current_repo: signals.set_current_repo,
        on_switch_repo: switch_callbacks.on_switch_repo,
        shadow_repos: signals.shadow_repos,
        on_list_shadows: sync_callbacks.on_list_shadows,
        repo_list: signals.repo_list,
        doc_version: signals.doc_version,
        set_doc_version: signals.set_doc_version,
        playback_version: signals.playback_version,
        set_playback_version: signals.set_playback_version,
        is_spectator: signals.is_spectator.into(),
        staged_changes: signals.staged_changes,
        unstaged_changes: signals.unstaged_changes,
        commit_history: signals.commit_history,
        on_get_changes: sc_callbacks.on_get_changes,
        on_stage_file: sc_callbacks.on_stage_file,
        on_stage_files: sc_callbacks.on_stage_files,
        on_unstage_file: sc_callbacks.on_unstage_file,
        on_unstage_files: sc_callbacks.on_unstage_files,
        on_discard_file: sc_callbacks.on_discard_file,
        on_commit: sc_callbacks.on_commit,
        on_get_history: sc_callbacks.on_get_history,
        diff_content: signals.diff_content,
        set_diff_content: signals.set_diff_content,
        on_get_doc_diff: sc_callbacks.on_get_doc_diff,
        commit_diff_result: signals.commit_diff_result,
        on_resolve_conflict: sc_callbacks.on_resolve_conflict,
        on_get_commit_diff: sc_callbacks.on_get_commit_diff,
        on_commit_and_push: sc_callbacks.on_commit_and_push,
        on_merge_peer: sync_callbacks.on_merge_peer,
        tree_nodes: signals.tree_nodes,
        set_explicit_home: signals.set_explicit_home,
        chat_messages: signals.chat_messages,
        set_chat_messages: signals.set_chat_messages,
        is_chat_streaming: signals.is_chat_streaming,
        set_is_chat_streaming: signals.set_is_chat_streaming,
        ai_mode: signals.ai_mode,
        set_ai_mode: signals.set_ai_mode,
    };

    provide_context(state.clone());
    provide::provide_sub_contexts(&state);
    provide_context(contexts::DashboardContext {
        metrics: signals.system_metrics,
    });
    state
}
