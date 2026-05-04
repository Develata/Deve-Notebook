//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::diff_session::DiffSessionWire;
use crate::hooks::use_core::write_gate::{RepoWriteGateState, repo_source_control_read_block};
use crate::storage::DegradedSyncMode;
use deve_core::models::DocId;
use leptos::prelude::*;

use super::effects_sc_feedback::show_file_op_feedback;

pub(super) struct FsRefreshSignals {
    pub expected_scope_nonce: u64,
    pub current_scope_nonce: ReadSignal<u64>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub load_state: ReadSignal<String>,
    pub is_spectator: Signal<bool>,
    pub handshake_ready: ReadSignal<bool>,
    pub pending_branch_switch: ReadSignal<Option<super::types::PendingBranchTarget>>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
    pub degraded_sync_mode: ReadSignal<Option<DegradedSyncMode>>,
    pub sync_banner: ReadSignal<Option<String>>,
    pub set_sync_banner: WriteSignal<Option<String>>,
    pub set_doc_list_request_id: WriteSignal<Option<String>>,
    pub set_tree_request_id: WriteSignal<Option<String>>,
}

pub(super) struct CommitRefreshSignals {
    pub expected_scope_nonce: u64,
    pub current_scope_nonce: ReadSignal<u64>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub load_state: ReadSignal<String>,
    pub is_spectator: Signal<bool>,
    pub handshake_ready: ReadSignal<bool>,
    pub pending_branch_switch: ReadSignal<Option<super::types::PendingBranchTarget>>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
    pub set_changes_request_id: WriteSignal<Option<String>>,
    pub set_commit_history_request_id: WriteSignal<Option<String>>,
}

pub(super) fn apply_doc_diff(
    doc_id: Option<DocId>,
    path: &str,
    old_content: &str,
    new_content: &str,
    set_diff: WriteSignal<Option<DiffSessionWire>>,
) {
    leptos::logging::log!("收到 Diff: {}", path);
    set_diff.set(Some(
        DiffSessionWire::new(
            path.to_string(),
            old_content.to_string(),
            new_content.to_string(),
        )
        .with_doc_id(doc_id),
    ));
    let ranges =
        deve_core::source_control::line_diff::compute_line_ranges(old_content, new_content);
    #[cfg(target_arch = "wasm32")]
    if let Ok(json) = serde_json::to_string(&ranges) {
        crate::editor::ffi::update_gutter_diff(&json);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = ranges;
}

pub(super) fn refresh_after_fs_change(
    path: &str,
    change_type: &str,
    has_conflict: bool,
    signals: FsRefreshSignals,
    schedule_refresh: &dyn Fn(),
    ws: &WsService,
) {
    let conflict_tag = if has_conflict { " [冲突]" } else { "" };
    if has_conflict || change_type != "dir_changed" {
        leptos::logging::log!("文件变更: {} ({}){}", path, change_type, conflict_tag);
    }
    show_file_op_feedback(
        path,
        change_type,
        has_conflict,
        signals.degraded_sync_mode,
        signals.sync_banner,
        signals.set_sync_banner,
    );
    let current_scope_nonce = signals.current_scope_nonce.get_untracked();
    let repo_id = signals.current_repo_id.get_untracked();
    let load_state = signals.load_state.get_untracked();
    let pending_branch_switch = signals.pending_branch_switch.get_untracked();
    let pending_repo_switch = signals.pending_repo_switch.get_untracked();
    if !source_control_refresh_allowed(
        signals.expected_scope_nonce,
        current_scope_nonce,
        RepoWriteGateState {
            connection_status: ws.status.get_untracked(),
            load_state: &load_state,
            is_read_only: signals.is_spectator.get_untracked(),
            handshake_ready: signals.handshake_ready.get_untracked(),
            writer_ready: ws.writer_ready_for(repo_id.as_deref(), Some(current_scope_nonce)),
            has_repo: repo_id.is_some(),
            pending_branch_switch: pending_branch_switch.is_some(),
            pending_repo_switch: pending_repo_switch.is_some(),
        },
    ) {
        return;
    }
    schedule_refresh();
    let request_id = uuid::Uuid::new_v4().to_string();
    signals
        .set_doc_list_request_id
        .set(Some(request_id.clone()));
    signals.set_tree_request_id.set(Some(request_id.clone()));
    ws.send(deve_core::protocol::ClientMessage::ListDocs {
        request_id,
        scope_nonce: Some(current_scope_nonce),
    });
}

pub(super) fn refresh_after_commit(commit_id: &str, signals: CommitRefreshSignals, ws: &WsService) {
    leptos::logging::log!("已提交: {}", commit_id);
    let current_scope_nonce = signals.current_scope_nonce.get_untracked();
    let repo_id = signals.current_repo_id.get_untracked();
    let load_state = signals.load_state.get_untracked();
    let pending_branch_switch = signals.pending_branch_switch.get_untracked();
    let pending_repo_switch = signals.pending_repo_switch.get_untracked();
    if !source_control_refresh_allowed(
        signals.expected_scope_nonce,
        current_scope_nonce,
        RepoWriteGateState {
            connection_status: ws.status.get_untracked(),
            load_state: &load_state,
            is_read_only: signals.is_spectator.get_untracked(),
            handshake_ready: signals.handshake_ready.get_untracked(),
            writer_ready: ws.writer_ready_for(repo_id.as_deref(), Some(current_scope_nonce)),
            has_repo: repo_id.is_some(),
            pending_branch_switch: pending_branch_switch.is_some(),
            pending_repo_switch: pending_repo_switch.is_some(),
        },
    ) {
        return;
    }
    let changes_request_id = uuid::Uuid::new_v4().to_string();
    signals
        .set_changes_request_id
        .set(Some(changes_request_id.clone()));
    ws.send(deve_core::protocol::ClientMessage::GetChanges {
        request_id: changes_request_id,
        scope_nonce: Some(current_scope_nonce),
    });
    let history_request_id = uuid::Uuid::new_v4().to_string();
    signals
        .set_commit_history_request_id
        .set(Some(history_request_id.clone()));
    ws.send(deve_core::protocol::ClientMessage::GetCommitHistory {
        request_id: history_request_id,
        limit: 50,
        scope_nonce: Some(current_scope_nonce),
    });
}

fn source_control_refresh_allowed(
    expected_scope_nonce: u64,
    current_scope_nonce: u64,
    gate_state: RepoWriteGateState<'_>,
) -> bool {
    expected_scope_nonce == current_scope_nonce
        && repo_source_control_read_block(gate_state).is_none()
}

#[cfg(test)]
mod tests {
    use super::{
        FsRefreshSignals, apply_doc_diff, refresh_after_fs_change, source_control_refresh_allowed,
    };
    use crate::api::{ConnectionStatus, WsService};
    use crate::hooks::use_core::write_gate::RepoWriteGateState;
    use deve_core::models::DocId;
    use deve_core::protocol::ClientMessage;
    use leptos::prelude::{GetUntracked, signal};
    use std::cell::Cell;

    fn gate_state(
        connection_status: ConnectionStatus,
        handshake_ready: bool,
        writer_ready: bool,
    ) -> RepoWriteGateState<'static> {
        RepoWriteGateState {
            connection_status,
            load_state: "ready",
            is_read_only: false,
            handshake_ready,
            writer_ready,
            has_repo: true,
            pending_branch_switch: false,
            pending_repo_switch: false,
        }
    }

    #[test]
    fn apply_doc_diff_preserves_doc_identity() {
        let (diff, set_diff) = signal(None);
        let doc_id = DocId::new();

        apply_doc_diff(Some(doc_id), "notes/a.md", "old", "new", set_diff);

        let session = diff.get_untracked().expect("diff session");
        assert_eq!(session.doc_id, Some(doc_id));
        assert_eq!(session.path, "notes/a.md");
    }

    #[test]
    fn commit_refresh_requires_current_scope_nonce() {
        assert!(!source_control_refresh_allowed(
            7,
            8,
            gate_state(ConnectionStatus::Connected, true, true),
        ));
    }

    #[test]
    fn commit_refresh_blocks_native_recovery_state() {
        assert!(!source_control_refresh_allowed(
            7,
            7,
            gate_state(ConnectionStatus::NativeReprobeRequired, true, true),
        ));
    }

    #[test]
    fn commit_refresh_requires_writer_ready() {
        assert!(!source_control_refresh_allowed(
            7,
            7,
            gate_state(ConnectionStatus::Connected, true, false),
        ));
    }

    #[test]
    fn commit_refresh_allows_ready_local_scope() {
        assert!(source_control_refresh_allowed(
            7,
            7,
            gate_state(ConnectionStatus::Connected, true, true),
        ));
    }

    fn run_fs_refresh(
        status: ConnectionStatus,
        writer_ready: bool,
    ) -> (bool, Option<String>, Option<String>, Vec<ClientMessage>) {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let ws = WsService::new_for_test(status);
        if writer_ready {
            ws.mark_writer_ready("repo-a", 7, "web-light-peer");
        }
        let (current_scope_nonce, _set_current_scope_nonce) = signal(7u64);
        let (current_repo_id, _set_current_repo_id) = signal(Some("repo-a".to_string()));
        let (load_state, _set_load_state) = signal("ready".to_string());
        let (is_spectator, _set_is_spectator) = signal(false);
        let (handshake_ready, _set_handshake_ready) = signal(true);
        let (pending_branch_switch, _set_pending_branch_switch) =
            signal(None::<super::super::types::PendingBranchTarget>);
        let (pending_repo_switch, _set_pending_repo_switch) = signal(None::<String>);
        let (degraded_sync_mode, _set_degraded_sync_mode) = signal(None);
        let (sync_banner, set_sync_banner) = signal(None::<String>);
        let (doc_list_request_id, set_doc_list_request_id) = signal(None::<String>);
        let (tree_request_id, set_tree_request_id) = signal(None::<String>);
        let scheduled = Cell::new(false);
        let schedule_refresh = || scheduled.set(true);

        refresh_after_fs_change(
            "notes/a.md",
            "modified",
            false,
            FsRefreshSignals {
                expected_scope_nonce: 7,
                current_scope_nonce,
                current_repo_id,
                load_state,
                is_spectator: is_spectator.into(),
                handshake_ready,
                pending_branch_switch,
                pending_repo_switch,
                degraded_sync_mode,
                sync_banner,
                set_sync_banner,
                set_doc_list_request_id,
                set_tree_request_id,
            },
            &schedule_refresh,
            &ws,
        );

        (
            scheduled.get(),
            doc_list_request_id.get_untracked(),
            tree_request_id.get_untracked(),
            ws.drain_sent_for_test(),
        )
    }

    #[test]
    fn fs_refresh_blocks_native_recovery_state() {
        let (scheduled, doc_list_request_id, tree_request_id, sent) =
            run_fs_refresh(ConnectionStatus::NativeReprobeRequired, true);

        assert!(!scheduled);
        assert_eq!(doc_list_request_id, None);
        assert_eq!(tree_request_id, None);
        assert!(sent.is_empty());
    }

    #[test]
    fn fs_refresh_sends_doc_list_when_read_gate_is_ready() {
        let (scheduled, doc_list_request_id, tree_request_id, sent) =
            run_fs_refresh(ConnectionStatus::Connected, true);

        assert!(scheduled);
        let request_id = doc_list_request_id.expect("doc list request");
        assert_eq!(tree_request_id.as_deref(), Some(request_id.as_str()));
        assert_eq!(sent.len(), 1);
        match &sent[0] {
            ClientMessage::ListDocs {
                request_id: sent_request_id,
                scope_nonce,
            } => {
                assert_eq!(sent_request_id, &request_id);
                assert_eq!(*scope_nonce, Some(7));
            }
            other => panic!("expected ListDocs, got {other:?}"),
        }
    }
}
