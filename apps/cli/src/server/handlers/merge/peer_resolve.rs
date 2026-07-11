//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Peer merge conflict resolution handlers.

use super::errors;
use super::peer_apply::{MergeWriteOutcome, write_merged_content};
use super::scope::resolve_local_write_scope;
use crate::server::repo_scope::ResolvedRepo;
use crate::server::{AppState, channel::DualChannel, session::PendingMergeConflict};
use deve_core::models::{DocId, MergeResolution};
use deve_core::protocol::MergeConflictAction;
use std::sync::Arc;

pub(super) async fn handle_resolve_merge_conflict(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut crate::server::session::WsSession,
    doc_id: DocId,
    action: MergeConflictAction,
    result_content: Option<String>,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let Some(scope) = resolve_local_write_scope(state, ch, session, scope_nonce) else {
        return;
    };
    let Some(pending) = session.pending_merge_conflict.take() else {
        errors::storage_not_found(ch, "No pending merge conflict", scope_nonce);
        return;
    };

    if !matches_pending_conflict(&pending, &scope, doc_id, scope_nonce) {
        session.pending_merge_conflict = Some(pending);
        errors::storage_conflict(ch, "Pending merge conflict target mismatch", scope_nonce);
        return;
    }

    let resolution = match &action {
        MergeConflictAction::AcceptCurrent => MergeResolution::AcceptCurrent,
        MergeConflictAction::AcceptIncoming => MergeResolution::AcceptIncoming,
        MergeConflictAction::AcceptBoth => MergeResolution::AcceptBoth,
    };
    let content = resolved_content(&pending, action, result_content);
    let outcome = write_merged_content(
        state,
        ch,
        &scope,
        &pending.preflight,
        &content,
        resolution,
        scope_nonce,
    );
    if should_restore_pending(outcome) {
        session.pending_merge_conflict = Some(pending);
    }
}

fn should_restore_pending(outcome: MergeWriteOutcome) -> bool {
    outcome == MergeWriteOutcome::CommitFailed
}

fn matches_pending_conflict(
    pending: &PendingMergeConflict,
    scope: &ResolvedRepo,
    doc_id: DocId,
    scope_nonce: Option<u64>,
) -> bool {
    pending.repo_id == scope.repo_id
        && pending.branch == scope.branch
        && pending.doc_id == doc_id
        && pending.scope_nonce == scope_nonce
}

fn resolved_content(
    pending: &PendingMergeConflict,
    action: MergeConflictAction,
    result_content: Option<String>,
) -> String {
    match action {
        MergeConflictAction::AcceptCurrent => pending.local_content.clone(),
        MergeConflictAction::AcceptIncoming => pending.incoming_content.clone(),
        MergeConflictAction::AcceptBoth => result_content
            .unwrap_or_else(|| accept_both(&pending.local_content, &pending.incoming_content)),
    }
}

fn accept_both(current: &str, incoming: &str) -> String {
    if current.is_empty() || incoming.is_empty() || current.ends_with('\n') {
        format!("{current}{incoming}")
    } else {
        format!("{current}\n{incoming}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolved_content_uses_selected_side_or_custom_result() {
        let pending = pending_conflict("local", "incoming");

        assert_eq!(
            resolved_content(&pending, MergeConflictAction::AcceptCurrent, None),
            "local"
        );
        assert_eq!(
            resolved_content(&pending, MergeConflictAction::AcceptIncoming, None),
            "incoming"
        );
        assert_eq!(
            resolved_content(
                &pending,
                MergeConflictAction::AcceptBoth,
                Some("manual result".into())
            ),
            "manual result"
        );
    }

    #[test]
    fn accept_both_preserves_line_boundary_without_extra_blank_line() {
        assert_eq!(accept_both("local", "incoming"), "local\nincoming");
        assert_eq!(accept_both("local\n", "incoming"), "local\nincoming");
        assert_eq!(accept_both("", "incoming"), "incoming");
    }

    #[test]
    fn pending_conflict_match_requires_doc_and_scope() {
        let pending = pending_conflict("local", "incoming");
        let scope = ResolvedRepo {
            repo_id: pending.repo_id,
            repo_name: "notes".into(),
            branch: pending.branch.clone(),
        };
        assert!(matches_pending_conflict(
            &pending,
            &scope,
            pending.doc_id,
            Some(11)
        ));
        assert!(!matches_pending_conflict(
            &pending,
            &scope,
            DocId::new(),
            Some(11)
        ));
        assert!(!matches_pending_conflict(
            &pending,
            &scope,
            pending.doc_id,
            Some(12)
        ));
        assert!(!matches_pending_conflict(
            &pending,
            &ResolvedRepo {
                repo_id: uuid::Uuid::new_v4(),
                repo_name: "notes".into(),
                branch: pending.branch.clone(),
            },
            pending.doc_id,
            Some(11)
        ));
    }

    #[test]
    fn committed_writeback_failure_does_not_restore_consumed_preflight() {
        assert!(should_restore_pending(MergeWriteOutcome::CommitFailed));
        assert!(!should_restore_pending(MergeWriteOutcome::Committed));
        assert!(!should_restore_pending(
            MergeWriteOutcome::CommittedWritebackFailed
        ));
    }

    fn pending_conflict(local_content: &str, incoming_content: &str) -> PendingMergeConflict {
        let repo_id = uuid::Uuid::new_v4();
        let doc_id = DocId::new();
        PendingMergeConflict {
            repo_id,
            branch: None,
            doc_id,
            scope_nonce: Some(11),
            local_content: local_content.into(),
            incoming_content: incoming_content.into(),
            preflight: crate::server::session::test_merge_preflight(
                repo_id,
                doc_id,
                local_content,
                incoming_content,
            ),
        }
    }
}
