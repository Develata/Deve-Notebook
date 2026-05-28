//! plan_ref:
//!   - 09_web_thin_client_ledger#web-edit-intent
//!   - 10_rendering#document-authority-bridge
//!
//! Server-authority outcome for the committed half of the document
//! write-confirmation contract.
//!
//! This mirrors the web client's `WriteConfirmation`
//! (`apps/web/src/runtime/document/write_state.rs`) so both sides of the
//! protocol speak the same lifecycle vocabulary. The server is the authority
//! and resolves synchronously, so it has no `Waiting` state. Rejections never
//! reach the ledger and are emitted by `edit_support::reject_edit` (the
//! `Rejected` half of the contract); this type models the post-commit
//! distinction `Committed` vs `WritebackFailed`, which is the new surface
//! Phase B introduced and the only place the server must choose between two
//! durable-but-differently-reconciled outcomes.

use deve_core::models::{DocId, Op};

use crate::server::channel::DualChannel;
use crate::server::repo_scope::ResolvedRepo;

use super::edit_support::{
    CommittedEdit, broadcast_and_ack_committed_edit, report_projection_writeback_fault,
};

/// Authority outcome of a client edit that was appended to the ledger.
///
/// Both variants mean the op is durable in the authority ledger at `seq`; they
/// differ only in whether the projection-workspace file has been reconciled —
/// exactly the web `Committed` / `WritebackFailed` distinction.
pub(super) enum CommitOutcome {
    /// Appended to the ledger at `seq` and the projection workspace reconciled.
    Committed { seq: u64 },
    /// Appended to the ledger at `seq` (durable) but the projection-workspace
    /// writeback failed afterwards; the on-disk file is not yet reconciled.
    /// `detail` is surfaced to the writer as a fault diagnostic.
    WritebackFailed { seq: u64, detail: String },
}

impl CommitOutcome {
    /// Authority-ledger seq the op committed at (durable in both variants).
    pub(super) fn ledger_seq(&self) -> u64 {
        match self {
            Self::Committed { seq } | Self::WritebackFailed { seq, .. } => *seq,
        }
    }
}

/// Identity of a committed write, paired with its [`CommitOutcome`] for emission.
pub(super) struct CommittedWrite<'a> {
    pub(super) ch: &'a DualChannel,
    pub(super) scope: &'a ResolvedRepo,
    pub(super) scope_nonce: u64,
    pub(super) doc_id: DocId,
    pub(super) op: Op,
    pub(super) client_id: u64,
    pub(super) client_op_id: u64,
}

/// Emit the committed half of the write-confirmation contract, derived from the
/// typed outcome: broadcast the new op to peers and ack the writer (the op is
/// durable in both variants), then — iff the workspace was not reconciled —
/// report the projection-writeback fault. Single wire-emission point for a
/// committed edit.
pub(super) fn emit_commit_outcome(write: CommittedWrite<'_>, outcome: CommitOutcome) {
    let CommittedWrite {
        ch,
        scope,
        scope_nonce,
        doc_id,
        op,
        client_id,
        client_op_id,
    } = write;
    broadcast_and_ack_committed_edit(
        ch,
        scope,
        CommittedEdit {
            scope_nonce,
            doc_id,
            local_seq: outcome.ledger_seq(),
            op,
            client_id,
            client_op_id,
        },
    );
    if let CommitOutcome::WritebackFailed { detail, .. } = outcome {
        report_projection_writeback_fault(ch, Some(scope_nonce), detail);
    }
}

#[cfg(test)]
mod tests {
    use super::CommitOutcome;

    #[test]
    fn committed_is_reconciled_and_durable_at_seq() {
        let outcome = CommitOutcome::Committed { seq: 7 };
        assert_eq!(outcome.ledger_seq(), 7);
        assert!(matches!(outcome, CommitOutcome::Committed { .. }));
    }

    #[test]
    fn writeback_failed_is_durable_at_seq_but_not_reconciled() {
        let outcome = CommitOutcome::WritebackFailed {
            seq: 9,
            detail: "disk full".into(),
        };
        // op is still durable in the ledger at `seq` ...
        assert_eq!(outcome.ledger_seq(), 9);
        // ... but the workspace file is not reconciled
        assert!(matches!(outcome, CommitOutcome::WritebackFailed { .. }));
    }
}
