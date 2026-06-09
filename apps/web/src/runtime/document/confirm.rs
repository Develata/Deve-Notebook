//! plan_ref:
//!   - 09_web_thin_client_ledger#web-edit-intent
//!   - 10_rendering#document-authority-bridge
//!
//! Write-confirmation resolution for the thin-client write path.
//!
//! This is the single typed entry that applies a server confirmation signal —
//! `ServerMessage::Ack` / an echoed `ConfirmedOp` (commit), or
//! `ServerMessage::EditRejected` (reject) — to the pending overlay. It
//! classifies the matched edit's outcome as a [`WriteConfirmation`] transition
//! and clears the resolved edit, so the three former ack/reject/echo call sites
//! (`effects/message_dispatch_write`, `effects/message_dispatch_protocol`,
//! `editor/sync/live`) share one lifecycle-typed resolution path instead of
//! each poking the overlay directly. Phase B step 3 of the runtime convergence
//! (`docs/report/runtime-convergence-audit-2026-05-28.md`).

use deve_core::models::{DocId, RepoId};

use super::pending::{
    PendingLocalEdits, clear_pending_edit_and_check_current_doc_empty, has_pending_edit,
};
use super::write_state::WriteConfirmation;

/// Outcome of applying one server confirmation signal to the pending overlay.
pub struct EditResolution {
    /// Lifecycle transition of the matched waiting edit, or `None` when no
    /// waiting edit matched this signal (a late / duplicate / out-of-scope
    /// signal — resolution is idempotent and leaves the overlay untouched).
    pub confirmation: Option<WriteConfirmation>,
    /// True iff this resolution emptied the current doc's in-scope pending set,
    /// releasing the navigation guard.
    pub clear_navigation: bool,
}

/// Apply a commit signal (`ServerMessage::Ack` or an echoed `ConfirmedOp`) for
/// edit `client_op_id`, recording the authority-ledger `seq`. A matching
/// waiting edit transitions `Waiting -> Committed { seq }`.
pub fn commit_pending_edit(
    pending: &mut PendingLocalEdits,
    current_doc: Option<DocId>,
    repo_id: Option<RepoId>,
    scope_nonce: Option<u64>,
    doc_id: DocId,
    client_op_id: u64,
    seq: u64,
) -> EditResolution {
    resolve(
        pending,
        current_doc,
        repo_id,
        scope_nonce,
        doc_id,
        client_op_id,
        |waiting| waiting.ack(seq),
    )
}

/// Apply a `ServerMessage::EditRejected` for edit `client_op_id`. A matching
/// waiting edit transitions `Waiting -> Rejected` (the op never entered the
/// ledger). Reject signals are not repo-scoped, so `repo_id` is left unfiltered.
pub fn reject_pending_edit(
    pending: &mut PendingLocalEdits,
    current_doc: Option<DocId>,
    scope_nonce: Option<u64>,
    doc_id: DocId,
    client_op_id: u64,
) -> EditResolution {
    resolve(
        pending,
        current_doc,
        None,
        scope_nonce,
        doc_id,
        client_op_id,
        WriteConfirmation::reject,
    )
}

/// Shared resolution: detect a matching waiting edit, clear it from the overlay
/// (recomputing the navigation guard), and classify the outcome via the typed
/// transition. Presence in the overlay is exactly the `Waiting` state — the
/// overlay only ever holds unconfirmed edits — so a match means the edit
/// transitions out of `Waiting`, and a miss means it was already resolved.
fn resolve(
    pending: &mut PendingLocalEdits,
    current_doc: Option<DocId>,
    repo_id: Option<RepoId>,
    scope_nonce: Option<u64>,
    doc_id: DocId,
    client_op_id: u64,
    transition: impl FnOnce(WriteConfirmation) -> WriteConfirmation,
) -> EditResolution {
    let matched = has_pending_edit(pending, repo_id, scope_nonce, doc_id, client_op_id);
    let clear_navigation = clear_pending_edit_and_check_current_doc_empty(
        pending,
        current_doc,
        repo_id,
        scope_nonce,
        doc_id,
        client_op_id,
    );
    let confirmation = matched.then(|| transition(WriteConfirmation::waiting()));
    EditResolution {
        confirmation,
        clear_navigation,
    }
}

#[cfg(test)]
mod tests {
    use super::super::pending::{PendingLocalEditInput, push_pending_edit};
    use super::*;
    use deve_core::models::Op;

    fn repo_id() -> RepoId {
        RepoId::from_u128(1)
    }

    fn seed(pending: &mut PendingLocalEdits, doc_id: DocId, scope_nonce: u64, client_op_id: u64) {
        push_pending_edit(
            pending,
            PendingLocalEditInput {
                repo_id: repo_id(),
                doc_id,
                scope_nonce,
                client_id: 11,
                client_op_id,
                base_version: 0,
                op: Op::Insert {
                    pos: 0,
                    content: "x".into(),
                },
            },
        );
    }

    #[test]
    fn commit_matches_waiting_edit_and_records_seq() {
        let doc_id = DocId::from_u128(10);
        let mut pending = PendingLocalEdits::new();
        seed(&mut pending, doc_id, 7, 13);

        let res = commit_pending_edit(
            &mut pending,
            Some(doc_id),
            Some(repo_id()),
            Some(7),
            doc_id,
            13,
            42,
        );

        assert_eq!(
            res.confirmation,
            Some(WriteConfirmation::Committed { seq: 42 })
        );
        assert!(res.clear_navigation);
        assert!(!pending.contains_key(&doc_id));
    }

    #[test]
    fn commit_without_match_is_idempotent_noop() {
        let doc_id = DocId::from_u128(11);
        let mut pending = PendingLocalEdits::new();
        seed(&mut pending, doc_id, 7, 13);

        // wrong client_op_id -> no waiting edit matched
        let res = commit_pending_edit(
            &mut pending,
            Some(doc_id),
            Some(repo_id()),
            Some(7),
            doc_id,
            99,
            42,
        );

        assert_eq!(res.confirmation, None);
        assert!(!res.clear_navigation);
        assert_eq!(pending.get(&doc_id).map(Vec::len), Some(1));
    }

    #[test]
    fn reject_matches_waiting_edit() {
        let doc_id = DocId::from_u128(12);
        let mut pending = PendingLocalEdits::new();
        seed(&mut pending, doc_id, 7, 13);

        let res = reject_pending_edit(&mut pending, Some(doc_id), Some(7), doc_id, 13);

        assert_eq!(res.confirmation, Some(WriteConfirmation::Rejected));
        assert!(res.confirmation.is_some_and(WriteConfirmation::is_failed));
        assert!(res.clear_navigation);
        assert!(!pending.contains_key(&doc_id));
    }

    #[test]
    fn commit_one_of_many_keeps_navigation_guard() {
        let doc_id = DocId::from_u128(13);
        let mut pending = PendingLocalEdits::new();
        seed(&mut pending, doc_id, 7, 13);
        seed(&mut pending, doc_id, 7, 14);

        let res = commit_pending_edit(
            &mut pending,
            Some(doc_id),
            Some(repo_id()),
            Some(7),
            doc_id,
            13,
            42,
        );

        assert_eq!(
            res.confirmation,
            Some(WriteConfirmation::Committed { seq: 42 })
        );
        // still one in-scope edit pending -> guard stays
        assert!(!res.clear_navigation);
        assert_eq!(pending.get(&doc_id).map(Vec::len), Some(1));
    }
}
