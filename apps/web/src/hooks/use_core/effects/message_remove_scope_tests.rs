//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 09_web_thin_client_ledger#write-readiness

use super::*;
use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::state::init_signals;
use crate::runtime::document::pending::{
    PendingLocalEditInput, pending_count_for_doc, push_pending_edit,
};
use crate::runtime::domain::PendingRepoSwitch;
use crate::runtime::remove_scope_partial::{
    REMOVE_SCOPE_PARTIAL_STAGE_TIMEOUT_MS, RemoveScopePartialKind,
};
use deve_core::models::{DocId, Op, RepoId};
use deve_core::protocol::{ClientMessage, RepoListEntry, RepoReadiness, ServerErrorCode};
use leptos::prelude::{GetUntracked, Owner, Set, Update};
use std::collections::VecDeque;

const OLD_SCOPE_NONCE: u64 = 7;
const REMOVE_SWITCH_NONCE: u64 = 8;

struct Harness {
    _owner: Owner,
    ws: WsService,
    signals: CoreSignals,
    removed_repo_id: RepoId,
    pending_doc_id: DocId,
}

impl Harness {
    fn initiator(connection_epoch: u64) -> Self {
        Self::new(connection_epoch, true)
    }

    fn observer(connection_epoch: u64) -> Self {
        Self::new(connection_epoch, false)
    }

    fn new(connection_epoch: u64, initiator: bool) -> Self {
        let owner = Owner::new();
        owner.set();
        let ws = WsService::new_with_incoming_for_test(
            ConnectionStatus::Connected,
            connection_epoch,
            VecDeque::new(),
        );
        let signals = init_signals(ws.status);
        let removed_repo_id = RepoId::new_v4();
        let pending_doc_id = DocId::new();

        signals.set_current_repo.set(Some("removed".to_string()));
        signals
            .set_current_repo_id
            .set(Some(removed_repo_id.to_string()));
        signals.set_current_scope_nonce.set(OLD_SCOPE_NONCE);
        signals.set_current_doc.set(Some(pending_doc_id));
        signals.set_repo_list.set(vec!["removed".to_string()]);
        signals
            .set_repo_entries
            .set(vec![repo_entry(removed_repo_id, "removed")]);
        signals.set_handshake_ready.set(true);
        signals.set_handshake_scope_nonce.set(Some(OLD_SCOPE_NONCE));
        if initiator {
            signals
                .set_pending_repo_switch
                .set(Some(PendingRepoSwitch::remove_current(
                    "removed",
                    REMOVE_SWITCH_NONCE,
                )));
        }
        signals.set_pending_local_edits.update(|pending| {
            push_pending_edit(
                pending,
                PendingLocalEditInput {
                    repo_id: removed_repo_id,
                    doc_id: pending_doc_id,
                    scope_nonce: OLD_SCOPE_NONCE,
                    client_id: 11,
                    client_op_id: 13,
                    base_version: 0,
                    op: Op::Insert {
                        pos: 0,
                        content: "pending".into(),
                    },
                },
            );
        });
        ws.mark_writer_ready(
            removed_repo_id.to_string(),
            OLD_SCOPE_NONCE,
            "web-light-peer",
        );

        Self {
            _owner: owner,
            ws,
            signals,
            removed_repo_id,
            pending_doc_id,
        }
    }

    fn capture_first_frame(&self) -> RepoId {
        let fallback_id = RepoId::new_v4();
        assert!(capture_repo_list(
            None,
            None,
            Some(REMOVE_SWITCH_NONCE),
            &["fallback".to_string()],
            &[repo_entry(fallback_id, "fallback")],
            &self.ws,
            self.signals,
        ));
        fallback_id
    }

    fn pending_count(&self) -> usize {
        pending_count_for_doc(
            &self.signals.pending_local_edits.get_untracked(),
            self.pending_doc_id,
        )
    }
}

fn repo_entry(repo_id: RepoId, name: &str) -> RepoListEntry {
    RepoListEntry {
        repo_id,
        display_alias: name.to_string(),
        alias_revision: 0,
        readiness: RepoReadiness::Mounted,
    }
}

#[test]
fn first_repo_list_frame_immediately_closes_writer_ready() {
    let harness = Harness::initiator(3);

    harness.capture_first_frame();

    assert!(!harness.ws.writer_ready_for(
        Some(&harness.removed_repo_id.to_string()),
        Some(OLD_SCOPE_NONCE),
    ));
    assert!(!harness.signals.handshake_ready.get_untracked());
    assert_eq!(
        harness.signals.current_repo.get_untracked().as_deref(),
        Some("removed")
    );
    assert_eq!(
        harness.signals.repo_list.get_untracked(),
        vec!["removed".to_string()]
    );
    assert_eq!(harness.pending_count(), 1);
    assert!(
        harness
            .signals
            .remove_scope_partial_stage
            .get_untracked()
            .is_some()
    );
}

#[test]
fn remove_current_fallback_failure_commits_no_scope() {
    let harness = Harness::initiator(4);
    let fallback_id = harness.capture_first_frame();

    assert!(capture_protocol_error(
        ServerErrorCode::ScRepoNotSelected,
        Some(REMOVE_SWITCH_NONCE),
        Some(REMOVE_SWITCH_NONCE),
        &harness.ws,
        Locale::En,
        harness.signals,
    ));

    assert!(harness.signals.current_repo.get_untracked().is_none());
    assert!(harness.signals.current_repo_id.get_untracked().is_none());
    assert!(
        harness
            .signals
            .pending_repo_switch
            .get_untracked()
            .is_none()
    );
    assert_eq!(
        harness.signals.repo_list.get_untracked(),
        vec!["fallback".to_string()]
    );
    assert_eq!(
        harness.signals.repo_entries.get_untracked(),
        vec![repo_entry(fallback_id, "fallback")]
    );
    assert_eq!(
        harness.signals.current_scope_nonce.get_untracked(),
        REMOVE_SWITCH_NONCE
    );
    assert!(
        harness
            .signals
            .explicit_repo_selection_required
            .get_untracked()
    );
    assert_eq!(harness.pending_count(), 1);
    assert!(
        harness
            .signals
            .remove_scope_partial_stage
            .get_untracked()
            .is_none()
    );
}

#[test]
fn remove_current_valid_fallback_commits_staged_repo_list_before_switch() {
    let harness = Harness::initiator(14);
    let fallback_id = harness.capture_first_frame();

    crate::hooks::use_core::effects::message_control::handle_repo_switched(
        None,
        "fallback".to_string(),
        fallback_id.to_string(),
        Some(REMOVE_SWITCH_NONCE),
        &harness.ws,
        harness.signals,
    );

    assert_eq!(
        harness.signals.current_repo.get_untracked().as_deref(),
        Some("fallback")
    );
    assert_eq!(
        harness.signals.current_repo_id.get_untracked().as_deref(),
        Some(fallback_id.to_string().as_str())
    );
    assert_eq!(
        harness.signals.repo_list.get_untracked(),
        vec!["fallback".to_string()]
    );
    assert_eq!(
        harness.signals.repo_entries.get_untracked(),
        vec![repo_entry(fallback_id, "fallback")]
    );
    assert_eq!(
        harness.signals.current_scope_nonce.get_untracked(),
        REMOVE_SWITCH_NONCE
    );
    assert!(
        harness
            .signals
            .remove_scope_partial_stage
            .get_untracked()
            .is_none()
    );
    assert!(
        harness
            .signals
            .pending_repo_switch
            .get_untracked()
            .is_none()
    );
    assert!(harness.ws.drain_connection_controls_for_test().is_empty());
}

#[test]
fn remove_current_fallback_identity_mismatch_retires_connection() {
    let harness = Harness::initiator(15);
    let fallback_id = harness.capture_first_frame();

    crate::hooks::use_core::effects::message_control::handle_repo_switched(
        None,
        "unexpected-alias".to_string(),
        fallback_id.to_string(),
        Some(REMOVE_SWITCH_NONCE),
        &harness.ws,
        harness.signals,
    );

    assert!(
        harness
            .signals
            .remove_scope_partial_stage
            .get_untracked()
            .is_none()
    );
    assert!(harness.signals.repo_list.get_untracked().is_empty());
    assert_eq!(harness.pending_count(), 1);
    assert_eq!(harness.ws.drain_connection_controls_for_test().len(), 1);
}

#[test]
fn remove_partial_stage_is_connection_epoch_bounded() {
    let harness = Harness::initiator(9);
    let stage = RemoveScopePartialStage::new(
        8,
        RemoveScopePartialKind::Initiator {
            switch_nonce: REMOVE_SWITCH_NONCE,
        },
        REMOVE_SWITCH_NONCE,
        harness.removed_repo_id,
        vec!["fallback".to_string()],
        vec![repo_entry(RepoId::new_v4(), "fallback")],
        0,
    );
    harness
        .signals
        .set_remove_scope_partial_stage
        .set(Some(stage));

    retire_stale_or_expired_at(&harness.ws, harness.signals, 1);

    assert!(
        harness
            .signals
            .remove_scope_partial_stage
            .get_untracked()
            .is_none()
    );
    assert!(harness.signals.repo_list.get_untracked().is_empty());
    assert!(matches!(
        harness.ws.drain_sent_for_test().as_slice(),
        [ClientMessage::ListRepos { .. }]
    ));
}

#[test]
fn remove_partial_mismatch_preserves_editor_pending_overlay() {
    let harness = Harness::initiator(5);
    harness.capture_first_frame();

    assert!(capture_protocol_error(
        ServerErrorCode::ScRepoNotSelected,
        Some(REMOVE_SWITCH_NONCE + 1),
        Some(REMOVE_SWITCH_NONCE),
        &harness.ws,
        Locale::En,
        harness.signals,
    ));

    assert_eq!(harness.pending_count(), 1);
    assert!(harness.signals.repo_list.get_untracked().is_empty());
    assert!(
        harness
            .signals
            .explicit_repo_selection_required
            .get_untracked()
    );
    assert_eq!(harness.ws.drain_connection_controls_for_test().len(), 1);
}

#[test]
fn remove_partial_timeout_uses_monotonic_deadline_and_retires_connection() {
    let harness = Harness::initiator(6);
    let stage = RemoveScopePartialStage::new(
        6,
        RemoveScopePartialKind::Initiator {
            switch_nonce: REMOVE_SWITCH_NONCE,
        },
        REMOVE_SWITCH_NONCE,
        harness.removed_repo_id,
        vec!["fallback".to_string()],
        vec![repo_entry(RepoId::new_v4(), "fallback")],
        100,
    );
    harness
        .signals
        .set_remove_scope_partial_stage
        .set(Some(stage));

    retire_stale_or_expired_at(
        &harness.ws,
        harness.signals,
        100 + REMOVE_SCOPE_PARTIAL_STAGE_TIMEOUT_MS - 1,
    );
    assert!(
        harness
            .signals
            .remove_scope_partial_stage
            .get_untracked()
            .is_some()
    );

    retire_stale_or_expired_at(
        &harness.ws,
        harness.signals,
        100 + REMOVE_SCOPE_PARTIAL_STAGE_TIMEOUT_MS,
    );
    assert!(
        harness
            .signals
            .remove_scope_partial_stage
            .get_untracked()
            .is_none()
    );
    assert_eq!(harness.pending_count(), 1);
    assert_eq!(harness.ws.drain_connection_controls_for_test().len(), 1);
}

#[test]
fn old_connection_second_frame_cannot_commit() {
    let harness = Harness::initiator(11);
    harness
        .signals
        .set_remove_scope_partial_stage
        .set(Some(RemoveScopePartialStage::new(
            10,
            RemoveScopePartialKind::Initiator {
                switch_nonce: REMOVE_SWITCH_NONCE,
            },
            REMOVE_SWITCH_NONCE,
            harness.removed_repo_id,
            vec!["fallback".to_string()],
            vec![repo_entry(RepoId::new_v4(), "fallback")],
            monotonic_now_ms(),
        )));

    assert!(capture_protocol_error(
        ServerErrorCode::ScRepoNotSelected,
        Some(REMOVE_SWITCH_NONCE),
        Some(REMOVE_SWITCH_NONCE),
        &harness.ws,
        Locale::En,
        harness.signals,
    ));

    assert!(harness.signals.repo_list.get_untracked().is_empty());
    assert_ne!(
        harness.signals.current_scope_nonce.get_untracked(),
        REMOVE_SWITCH_NONCE
    );
    assert!(matches!(
        harness.ws.drain_sent_for_test().as_slice(),
        [ClientMessage::ListRepos { .. }]
    ));
}

#[test]
fn observer_invalidation_does_not_reuse_initiator_switch_nonce() {
    let harness = Harness::observer(12);
    harness.capture_first_frame();
    assert!(matches!(
        harness
            .signals
            .remove_scope_partial_stage
            .get_untracked()
            .map(|stage| stage.kind),
        Some(RemoveScopePartialKind::Observer)
    ));

    assert!(capture_protocol_error(
        ServerErrorCode::ScRepoNotSelected,
        Some(REMOVE_SWITCH_NONCE),
        Some(REMOVE_SWITCH_NONCE),
        &harness.ws,
        Locale::En,
        harness.signals,
    ));

    assert!(harness.signals.repo_list.get_untracked().is_empty());
    assert_eq!(
        harness.signals.current_scope_nonce.get_untracked(),
        OLD_SCOPE_NONCE
    );
    assert_eq!(harness.pending_count(), 1);
    assert_eq!(harness.ws.drain_connection_controls_for_test().len(), 1);
}
