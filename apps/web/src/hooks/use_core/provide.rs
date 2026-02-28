// apps/web/src/hooks/use_core/provide.rs
//! 将 CoreState 分发为 6 个独立子上下文，各组件按需拉取。

use super::contexts::*;
use super::types::CoreState;
use leptos::prelude::*;

/// 从已构建的 CoreState 提取字段，构造并提供 6 个独立上下文。
///
/// ## Invariant
/// 子上下文与 CoreState 共享同一组 Signal —— 无额外分配。
pub fn provide_sub_contexts(state: &CoreState) {
    provide_context(DocContext {
        docs: state.docs,
        current_doc: state.current_doc,
        set_current_doc: state.set_current_doc,
        tree_nodes: state.tree_nodes,
        on_doc_select: state.on_doc_select,
        on_doc_create: state.on_doc_create,
        on_doc_rename: state.on_doc_rename,
        on_doc_delete: state.on_doc_delete,
        on_doc_copy: state.on_doc_copy,
        on_doc_move: state.on_doc_move,
        search_results: state.search_results,
        on_search: state.on_search,
    });
    provide_context(EditorContext {
        stats: state.stats,
        on_stats: state.on_stats,
        load_state: state.load_state,
        set_load_state: state.set_load_state,
        load_progress: state.load_progress,
        set_load_progress: state.set_load_progress,
        load_eta_ms: state.load_eta_ms,
        set_load_eta_ms: state.set_load_eta_ms,
        doc_version: state.doc_version,
        set_doc_version: state.set_doc_version,
        playback_version: state.playback_version,
        set_playback_version: state.set_playback_version,
        is_spectator: state.is_spectator,
    });
    provide_context(ChatContext {
        messages: state.chat_messages,
        set_messages: state.set_chat_messages,
        is_streaming: state.is_chat_streaming,
        set_is_streaming: state.set_is_chat_streaming,
        ai_mode: state.ai_mode,
        set_ai_mode: state.set_ai_mode,
        plugin_last_response: state.plugin_last_response,
        on_plugin_call: state.on_plugin_call,
    });
    provide_context(SyncMergeContext {
        sync_mode: state.sync_mode,
        pending_ops_count: state.pending_ops_count,
        pending_ops_previews: state.pending_ops_previews,
        on_get_sync_mode: state.on_get_sync_mode,
        on_set_sync_mode: state.on_set_sync_mode,
        on_get_pending_ops: state.on_get_pending_ops,
        on_confirm_merge: state.on_confirm_merge,
        on_discard_pending: state.on_discard_pending,
        on_merge_peer: state.on_merge_peer,
    });
    provide_context(SourceControlContext {
        staged_changes: state.staged_changes,
        unstaged_changes: state.unstaged_changes,
        commit_history: state.commit_history,
        on_get_changes: state.on_get_changes,
        on_stage_file: state.on_stage_file,
        on_stage_files: state.on_stage_files,
        on_unstage_file: state.on_unstage_file,
        on_unstage_files: state.on_unstage_files,
        on_discard_file: state.on_discard_file,
        on_commit: state.on_commit,
        on_get_history: state.on_get_history,
        diff_content: state.diff_content,
        set_diff_content: state.set_diff_content,
        on_get_doc_diff: state.on_get_doc_diff,
    });
    provide_context(BranchContext {
        active_branch: state.active_branch,
        set_active_branch: state.set_active_branch,
        on_switch_branch: state.on_switch_branch,
        current_repo: state.current_repo,
        set_current_repo: state.set_current_repo,
        on_switch_repo: state.on_switch_repo,
        shadow_repos: state.shadow_repos,
        on_list_shadows: state.on_list_shadows,
        repo_list: state.repo_list,
    });
}
