//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::{ExternalChangesMutationError, WsService};
use crate::i18n::{Locale, t};
use crate::runtime::{
    CoreRuntimeClients,
    document_client::DocumentClient,
    external_changes_client::{
        ExternalChangesClient, ExternalChangesHttpScope,
        create_external_changes_mutation_callbacks, create_external_changes_refresh_callback,
    },
    rendering_client::RenderingClient,
    scope_client::ScopeClient,
    session_client::SessionClient,
    source_control_client::SourceControlClient,
};
use leptos::prelude::{Callback, GetUntracked, RwSignal, Set, Update, WriteSignal, signal};

use super::super::CoreState;
use super::super::callbacks_switch::SwitchCallbacks;
use super::doc::DocStateSection;
use super::runtime::RuntimeStateSection;
use super::source_control::SourceControlStateSection;
use super::sync::SyncStateSection;

pub(super) fn assemble_core_state(
    ws: WsService,
    doc: DocStateSection,
    runtime: RuntimeStateSection,
    sync: SyncStateSection,
    sc: SourceControlStateSection,
    switch: SwitchCallbacks,
    locale: RwSignal<Locale>,
) -> CoreState {
    let on_retry_peer_registration = build_retry_peer_registration_callback(
        ws.clone(),
        RetryPeerRegistrationSignals {
            set_handshake_ready: sync.set_handshake_ready,
            set_handshake_scope_nonce: sync.set_handshake_scope_nonce,
            set_handshake_retry_nonce: sync.set_handshake_retry_nonce,
            set_repo_list_request_id: sync.set_repo_list_request_id,
            set_doc_list_request_id: sync.set_doc_list_request_id,
            set_tree_request_id: sync.set_tree_request_id,
        },
    );
    let external_changes_error = {
        let set_sync_banner = runtime.set_sync_banner;
        let ws = ws.clone();
        let current_repo_id = sync.current_repo_id;
        let current_scope_nonce = sync.current_scope_nonce;
        Callback::new(move |error: ExternalChangesMutationError| {
            let workspace_ingestion_unavailable =
                record_external_changes_workspace_ingestion_blocker(
                    &ws,
                    current_repo_id.get_untracked(),
                    current_scope_nonce.get_untracked(),
                    &error,
                );
            leptos::logging::error!(
                "External Changes request failed: status={:?} code={:?}",
                external_changes_error_status(&error),
                error.server_error().map(|error| error.code)
            );
            if !workspace_ingestion_unavailable {
                set_sync_banner.set(Some(external_changes_error_message(
                    locale.get_untracked(),
                    &error,
                )));
            }
        })
    };
    let external_changes_scope = ExternalChangesHttpScope {
        current_connection_epoch: ws.connection_epoch,
        current_repo_id: sync.current_repo_id,
        current_scope_nonce: sync.current_scope_nonce,
    };
    let (external_staged_changes, set_external_staged_changes) =
        signal(Vec::<deve_core::source_control::ChangeEntry>::new());
    let (external_unstaged_changes, set_external_unstaged_changes) =
        signal(Vec::<deve_core::source_control::ChangeEntry>::new());
    let external_changes_refresh = create_external_changes_refresh_callback(
        external_changes_scope,
        set_external_staged_changes,
        set_external_unstaged_changes,
        external_changes_error.clone(),
    );
    let external_changes_mutations = create_external_changes_mutation_callbacks(
        external_changes_scope,
        ws.clone(),
        external_changes_refresh.clone(),
        external_changes_error,
    );
    let runtime_clients = CoreRuntimeClients {
        session: SessionClient {
            ws: ws.clone(),
            connection_status: ws.status,
            status_text: runtime.status_text,
            sync_banner: runtime.sync_banner,
            set_sync_banner: runtime.set_sync_banner,
            handshake_ready: sync.handshake_ready,
            handshake_scope_nonce: sync.handshake_scope_nonce,
            on_retry_peer_registration: on_retry_peer_registration.clone(),
        },
        scope: ScopeClient {
            current_doc: doc.current_doc,
            current_repo: sync.current_repo,
            current_repo_id: sync.current_repo_id,
            current_scope_nonce: sync.current_scope_nonce,
            active_branch: sync.active_branch,
            set_active_branch: sync.set_active_branch,
            pending_repo_switch: sync.pending_repo_switch,
            on_switch_repo: switch.on_switch_repo.clone(),
            on_create_repo: switch.on_create_repo.clone(),
            on_rename_repo: switch.on_rename_repo.clone(),
            on_remove_repo: switch.on_remove_repo.clone(),
            on_switch_branch: switch.on_switch_branch.clone(),
            set_current_repo: sync.set_current_repo,
            set_current_repo_id: sync.set_current_repo_id,
            shadow_repos: sync.shadow_repos,
            on_list_shadows: sync.on_list_shadows,
            repo_list: sync.repo_list,
            repo_entries: sync.repo_entries,
            is_spectator: sync.is_spectator,
        },
        document: DocumentClient {
            docs: doc.docs,
            current_doc: doc.current_doc,
            set_current_doc: doc.set_current_doc,
            set_explicit_home: doc.set_explicit_home,
            pending_local_edits: doc.pending_local_edits,
            set_pending_local_edits: doc.set_pending_local_edits,
            on_doc_select: doc.on_doc_select.clone(),
            on_doc_create: doc.on_doc_create.clone(),
            on_doc_rename: doc.on_doc_rename.clone(),
            on_doc_delete: doc.on_doc_delete.clone(),
            on_doc_copy: doc.on_doc_copy.clone(),
            on_doc_move: doc.on_doc_move.clone(),
            tree_nodes: doc.tree_nodes,
        },
        source_control: SourceControlClient {
            staged_changes: sc.staged_changes,
            unstaged_changes: sc.unstaged_changes,
            confirmed_changes: sc.confirmed_changes,
            commit_history: sc.commit_history,
            commit_history_request_id: sc.commit_history_request_id,
            commit_diff_request_id: sc.commit_diff_request_id,
            set_commit_diff_request_id: sc.set_commit_diff_request_id,
            on_get_changes: sc.on_get_changes.clone(),
            on_stage_file: sc.on_stage_file.clone(),
            on_stage_files: sc.on_stage_files.clone(),
            on_unstage_file: sc.on_unstage_file.clone(),
            on_unstage_files: sc.on_unstage_files.clone(),
            on_discard_file: sc.on_discard_file.clone(),
            on_commit: sc.on_commit.clone(),
            on_get_history: sc.on_get_history.clone(),
            diff_content: sc.diff_content,
            set_diff_content: sc.set_diff_content,
            on_get_doc_diff: sc.on_get_doc_diff.clone(),
            commit_diff_result: sc.commit_diff_result,
            set_commit_diff_result: sc.set_commit_diff_result,
            on_resolve_conflict: sc.on_resolve_conflict.clone(),
            on_get_commit_diff: sc.on_get_commit_diff.clone(),
            on_commit_and_push: sc.on_commit_and_push.clone(),
        },
        external_changes: ExternalChangesClient {
            staged_changes: external_staged_changes,
            unstaged_changes: external_unstaged_changes,
            on_get_changes: external_changes_refresh,
            on_stage_file: external_changes_mutations.on_stage_file,
            on_stage_files: external_changes_mutations.on_stage_files,
            on_unstage_file: external_changes_mutations.on_unstage_file,
            on_unstage_files: external_changes_mutations.on_unstage_files,
            on_discard_file: external_changes_mutations.on_discard_file,
            on_apply_to_ledger: external_changes_mutations.on_apply_to_ledger,
            on_get_doc_diff: sc.on_get_doc_diff.clone(),
        },
        rendering: RenderingClient {
            stats: runtime.stats,
            on_stats: runtime.on_stats.clone(),
            load_state: runtime.load_state,
            set_load_state: runtime.set_load_state,
            load_progress: runtime.load_progress,
            set_load_progress: runtime.set_load_progress,
            load_eta_ms: runtime.load_eta_ms,
            set_load_eta_ms: runtime.set_load_eta_ms,
        },
    };

    CoreState {
        ws,
        runtime_clients,
        docs: doc.docs,
        current_doc: doc.current_doc,
        set_current_doc: doc.set_current_doc,
        status_text: runtime.status_text,
        sync_banner: runtime.sync_banner,
        set_sync_banner: runtime.set_sync_banner,
        stats: runtime.stats,
        peers: sync.peers,
        handshake_ready: sync.handshake_ready,
        handshake_scope_nonce: sync.handshake_scope_nonce,
        on_retry_peer_registration,
        pending_local_edits: doc.pending_local_edits,
        set_pending_local_edits: doc.set_pending_local_edits,
        pending_navigation: doc.pending_navigation,
        set_pending_navigation: doc.set_pending_navigation,
        on_doc_select: doc.on_doc_select,
        on_doc_create: doc.on_doc_create,
        on_doc_rename: doc.on_doc_rename,
        on_doc_delete: doc.on_doc_delete,
        on_doc_copy: doc.on_doc_copy,
        on_doc_move: doc.on_doc_move,
        on_stats: runtime.on_stats,
        plugin_last_response: runtime.plugin_last_response,
        plugin_request_ids: runtime.plugin_request_ids,
        on_plugin_call: runtime.on_plugin_call,
        search_results: runtime.search_results,
        on_search: runtime.on_search,
        load_state: runtime.load_state,
        set_load_state: runtime.set_load_state,
        load_progress: runtime.load_progress,
        set_load_progress: runtime.set_load_progress,
        load_eta_ms: runtime.load_eta_ms,
        set_load_eta_ms: runtime.set_load_eta_ms,
        sync_mode: sync.sync_mode,
        pending_ops_count: sync.pending_ops_count,
        pending_ops_previews: sync.pending_ops_previews,
        on_get_sync_mode: sync.on_get_sync_mode,
        on_set_sync_mode: sync.on_set_sync_mode,
        on_get_pending_ops: sync.on_get_pending_ops,
        on_confirm_merge: sync.on_confirm_merge,
        on_discard_pending: sync.on_discard_pending,
        active_branch: sync.active_branch,
        set_active_branch: sync.set_active_branch,
        pending_branch_switch: sync.pending_branch_switch,
        on_switch_branch: switch.on_switch_branch,
        current_repo: sync.current_repo,
        set_current_repo: sync.set_current_repo,
        current_repo_id: sync.current_repo_id,
        set_current_repo_id: sync.set_current_repo_id,
        current_scope_nonce: sync.current_scope_nonce,
        pending_repo_switch: sync.pending_repo_switch,
        on_switch_repo: switch.on_switch_repo,
        on_create_repo: switch.on_create_repo,
        on_rename_repo: switch.on_rename_repo,
        on_remove_repo: switch.on_remove_repo,
        shadow_repos: sync.shadow_repos,
        on_list_shadows: sync.on_list_shadows,
        repo_list: sync.repo_list,
        repo_entries: sync.repo_entries,
        doc_version: sync.doc_version,
        set_doc_version: sync.set_doc_version,
        playback_version: sync.playback_version,
        set_playback_version: sync.set_playback_version,
        is_spectator: sync.is_spectator,
        staged_changes: sc.staged_changes,
        unstaged_changes: sc.unstaged_changes,
        confirmed_changes: sc.confirmed_changes,
        commit_history: sc.commit_history,
        commit_history_request_id: sc.commit_history_request_id,
        commit_diff_request_id: sc.commit_diff_request_id,
        set_commit_diff_request_id: sc.set_commit_diff_request_id,
        on_get_changes: sc.on_get_changes,
        on_stage_file: sc.on_stage_file,
        on_stage_files: sc.on_stage_files,
        on_unstage_file: sc.on_unstage_file,
        on_unstage_files: sc.on_unstage_files,
        on_discard_file: sc.on_discard_file,
        on_commit: sc.on_commit,
        on_get_history: sc.on_get_history,
        diff_content: sc.diff_content,
        set_diff_content: sc.set_diff_content,
        on_get_doc_diff: sc.on_get_doc_diff,
        commit_diff_result: sc.commit_diff_result,
        set_commit_diff_result: sc.set_commit_diff_result,
        source_control_notice: sc.source_control_notice,
        set_source_control_notice: sc.set_source_control_notice,
        on_resolve_conflict: sc.on_resolve_conflict,
        on_get_commit_diff: sc.on_get_commit_diff,
        on_commit_and_push: sc.on_commit_and_push,
        on_merge_peer: sync.on_merge_peer,
        tree_nodes: doc.tree_nodes,
        set_explicit_home: doc.set_explicit_home,
        chat_messages: runtime.chat_messages,
        set_chat_messages: runtime.set_chat_messages,
        is_chat_streaming: runtime.is_chat_streaming,
        set_is_chat_streaming: runtime.set_is_chat_streaming,
        ai_mode: runtime.ai_mode,
        set_ai_mode: runtime.set_ai_mode,
    }
}

fn external_changes_error_message(locale: Locale, error: &ExternalChangesMutationError) -> String {
    let base = t::external_changes::request_failed(locale);
    match error {
        ExternalChangesMutationError::Rejected {
            error: Some(error), ..
        } => format!("{base}: {}", t::server_error::message(locale, error.code)),
        ExternalChangesMutationError::Rejected { status, .. } => format!("{base}: HTTP {status}"),
        ExternalChangesMutationError::RequestBuild
        | ExternalChangesMutationError::RequestFailed => base.to_string(),
    }
}

fn external_changes_error_status(error: &ExternalChangesMutationError) -> Option<u16> {
    match error {
        ExternalChangesMutationError::Rejected { status, .. } => Some(*status),
        ExternalChangesMutationError::RequestBuild
        | ExternalChangesMutationError::RequestFailed => None,
    }
}

fn is_workspace_ingestion_unavailable(error: &ExternalChangesMutationError) -> bool {
    error.server_error().is_some_and(|error| {
        error.code == deve_core::protocol::ServerErrorCode::StorageWorkspaceIngestionUnavailable
    })
}

fn record_external_changes_workspace_ingestion_blocker(
    ws: &WsService,
    repo_id: Option<String>,
    scope_nonce: u64,
    error: &ExternalChangesMutationError,
) -> bool {
    if !is_workspace_ingestion_unavailable(error) {
        return false;
    }
    let Some(repo_id) = repo_id else {
        return false;
    };
    ws.mark_workspace_ingestion_unavailable(repo_id, scope_nonce);
    true
}

#[derive(Clone, Copy)]
struct RetryPeerRegistrationSignals {
    set_handshake_ready: WriteSignal<bool>,
    set_handshake_scope_nonce: WriteSignal<Option<u64>>,
    set_handshake_retry_nonce: WriteSignal<u64>,
    set_repo_list_request_id: WriteSignal<Option<String>>,
    set_doc_list_request_id: WriteSignal<Option<String>>,
    set_tree_request_id: WriteSignal<Option<String>>,
}

fn build_retry_peer_registration_callback(
    ws: WsService,
    signals: RetryPeerRegistrationSignals,
) -> Callback<()> {
    Callback::new(move |_: ()| {
        ws.clear_writer_ready();
        signals.set_handshake_ready.set(false);
        signals.set_handshake_scope_nonce.set(None);
        signals.set_repo_list_request_id.set(None);
        signals.set_doc_list_request_id.set(None);
        signals.set_tree_request_id.set(None);
        signals.set_handshake_retry_nonce.update(|nonce| {
            *nonce = nonce.wrapping_add(1);
        });
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ConnectionStatus;
    use leptos::prelude::{Callable, GetUntracked, signal};

    #[test]
    fn retry_peer_registration_clears_stale_writer_and_bumps_retry_nonce() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        ws.mark_writer_ready("repo-a", 7, "web-light-peer");
        let (handshake_ready, set_handshake_ready) = signal(true);
        let (handshake_scope_nonce, set_handshake_scope_nonce) = signal(Some(7u64));
        let (handshake_retry_nonce, set_handshake_retry_nonce) = signal(0u64);
        let (repo_list_request_id, set_repo_list_request_id) = signal(Some("repo".to_string()));
        let (doc_list_request_id, set_doc_list_request_id) = signal(Some("doc".to_string()));
        let (tree_request_id, set_tree_request_id) = signal(Some("tree".to_string()));

        let retry = build_retry_peer_registration_callback(
            ws.clone(),
            RetryPeerRegistrationSignals {
                set_handshake_ready,
                set_handshake_scope_nonce,
                set_handshake_retry_nonce,
                set_repo_list_request_id,
                set_doc_list_request_id,
                set_tree_request_id,
            },
        );

        retry.run(());

        assert!(!ws.writer_ready_for(Some("repo-a"), Some(7)));
        assert!(!handshake_ready.get_untracked());
        assert_eq!(handshake_scope_nonce.get_untracked(), None);
        assert_eq!(handshake_retry_nonce.get_untracked(), 1);
        assert_eq!(repo_list_request_id.get_untracked(), None);
        assert_eq!(doc_list_request_id.get_untracked(), None);
        assert_eq!(tree_request_id.get_untracked(), None);
    }

    #[test]
    fn external_changes_error_message_uses_typed_code_and_ignores_backend_detail() {
        assert_eq!(
            external_changes_error_message(
                Locale::Zh,
                &ExternalChangesMutationError::Rejected {
                    status: 409,
                    error: Some(deve_core::protocol::ServerError::with_detail(
                        deve_core::protocol::ServerErrorCode::ScPendingNotFound,
                        "pending target vanished",
                    )),
                },
            ),
            "外部修改请求失败: 待处理变更不存在"
        );
        assert_eq!(
            external_changes_error_message(
                Locale::En,
                &ExternalChangesMutationError::Rejected {
                    status: 409,
                    error: None,
                },
            ),
            "External Changes request failed: HTTP 409"
        );
    }

    #[test]
    fn external_changes_http_typed_503_binds_workspace_ingestion_blocker() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        let error = ExternalChangesMutationError::Rejected {
            status: 503,
            error: Some(deve_core::protocol::ServerError::with_detail(
                deve_core::protocol::ServerErrorCode::StorageWorkspaceIngestionUnavailable,
                "CANARY_PRIVATE_BACKEND_DETAIL",
            )),
        };

        assert!(record_external_changes_workspace_ingestion_blocker(
            &ws,
            Some("repo-a".into()),
            7,
            &error,
        ));
        assert!(ws.workspace_ingestion_blocked_for_untracked(Some("repo-a"), Some(7)));
        assert!(!ws.workspace_ingestion_blocked_for_untracked(Some("repo-b"), Some(7)));
    }
}
