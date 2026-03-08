use crate::api::WsService;
use leptos::prelude::*;

use super::CoreState;
use super::callbacks::{
    DocCallbacks, MiscCallbacks, SourceControlCallbacks, SwitchCallbacks, SyncCallbacks,
};
use super::state::CoreSignals;

pub(super) struct CoreStateCallbacks {
    doc: DocCallbacks,
    sync: SyncCallbacks,
    sc: SourceControlCallbacks,
    misc: MiscCallbacks,
    switch: SwitchCallbacks,
}

impl CoreStateCallbacks {
    pub(super) fn new(
        doc: DocCallbacks,
        sync: SyncCallbacks,
        sc: SourceControlCallbacks,
        misc: MiscCallbacks,
        switch: SwitchCallbacks,
    ) -> Self {
        Self {
            doc,
            sync,
            sc,
            misc,
            switch,
        }
    }
}

pub(super) fn build_core_state(
    ws: WsService,
    signals: &CoreSignals,
    status_text: Signal<String>,
    callbacks: CoreStateCallbacks,
) -> CoreState {
    let CoreStateCallbacks {
        doc,
        sync,
        sc,
        misc,
        switch,
    } = callbacks;

    CoreState {
        ws,
        docs: signals.docs,
        current_doc: signals.current_doc,
        set_current_doc: signals.set_current_doc,
        status_text,
        sync_banner: signals.sync_banner.into(),
        stats: signals.stats,
        peers: signals.peers,
        handshake_ready: signals.handshake_ready,
        on_doc_select: doc.on_doc_select,
        on_doc_create: doc.on_doc_create,
        on_doc_rename: doc.on_doc_rename,
        on_doc_delete: doc.on_doc_delete,
        on_doc_copy: doc.on_doc_copy,
        on_doc_move: doc.on_doc_move,
        on_stats: misc.on_stats,
        plugin_last_response: signals.plugin_response,
        on_plugin_call: misc.on_plugin_call,
        search_results: signals.search_results,
        on_search: misc.on_search,
        load_state: signals.load_state,
        set_load_state: signals.set_load_state,
        load_progress: signals.load_progress,
        set_load_progress: signals.set_load_progress,
        load_eta_ms: signals.load_eta_ms,
        set_load_eta_ms: signals.set_load_eta_ms,
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
        on_switch_branch: switch.on_switch_branch,
        current_repo: signals.current_repo,
        set_current_repo: signals.set_current_repo,
        current_repo_id: signals.current_repo_id,
        set_current_repo_id: signals.set_current_repo_id,
        on_switch_repo: switch.on_switch_repo,
        shadow_repos: signals.shadow_repos,
        on_list_shadows: sync.on_list_shadows,
        repo_list: signals.repo_list,
        doc_version: signals.doc_version,
        set_doc_version: signals.set_doc_version,
        playback_version: signals.playback_version,
        set_playback_version: signals.set_playback_version,
        is_spectator: signals.is_spectator.into(),
        staged_changes: signals.staged_changes,
        unstaged_changes: signals.unstaged_changes,
        commit_history: signals.commit_history,
        on_get_changes: sc.on_get_changes,
        on_stage_file: sc.on_stage_file,
        on_stage_files: sc.on_stage_files,
        on_unstage_file: sc.on_unstage_file,
        on_unstage_files: sc.on_unstage_files,
        on_discard_file: sc.on_discard_file,
        on_commit: sc.on_commit,
        on_get_history: sc.on_get_history,
        diff_content: signals.diff_content,
        set_diff_content: signals.set_diff_content,
        on_get_doc_diff: sc.on_get_doc_diff,
        commit_diff_result: signals.commit_diff_result,
        on_resolve_conflict: sc.on_resolve_conflict,
        on_get_commit_diff: sc.on_get_commit_diff,
        on_commit_and_push: sc.on_commit_and_push,
        on_merge_peer: sync.on_merge_peer,
        tree_nodes: signals.tree_nodes,
        set_explicit_home: signals.set_explicit_home,
        chat_messages: signals.chat_messages,
        set_chat_messages: signals.set_chat_messages,
        is_chat_streaming: signals.is_chat_streaming,
        set_is_chat_streaming: signals.set_is_chat_streaming,
        ai_mode: signals.ai_mode,
        set_ai_mode: signals.set_ai_mode,
    }
}
