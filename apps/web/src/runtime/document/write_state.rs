//! plan_ref:
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
//! Formalized write-confirmation lifecycle for a single client edit intent
//! (`client_op_id`) in the thin-client write path.
//!
//! Phase B of the runtime convergence (see
//! `docs/report/runtime-convergence-audit-2026-05-28.md` and
//! `docs/tasks/20_web_thin_client_ledger_migration.md`) introduces this typed
//! contract first; the pending overlay, history resend, and ack/reject
//! handling are migrated onto it in subsequent steps. Until then the type is
//! scaffolding for those moves.
#![allow(dead_code)]

/// Lifecycle of one client edit intent in the thin-client write path.
///
/// Source signals (server protocol, `07_network`):
/// - sent, no response yet                     -> [`WriteConfirmation::Waiting`]
/// - `ServerMessage::Ack`                       -> [`WriteConfirmation::Committed`]
/// - `ServerMessage::EditRejected`              -> [`WriteConfirmation::Rejected`]
/// - committed but workspace writeback failed   -> [`WriteConfirmation::WritebackFailed`]
///
/// `Committed` and `WritebackFailed` both mean the op is durable in the
/// authority ledger; they differ only in whether the projection-workspace file
/// has been reconciled. `Rejected` means the op never entered the ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteConfirmation {
    /// Sent to the server; awaiting `Ack` or `EditRejected`.
    Waiting,
    /// `Ack` received — appended to the authority ledger at `seq`.
    Committed { seq: u64 },
    /// `EditRejected` received — the op never entered the ledger.
    Rejected,
    /// Committed at `seq` but the projection-workspace writeback failed; the
    /// authoritative op is durable, the on-disk file is not yet reconciled.
    WritebackFailed { seq: u64 },
}

impl WriteConfirmation {
    /// Initial state for a freshly sent edit intent.
    pub fn waiting() -> Self {
        Self::Waiting
    }

    /// Still awaiting a server response.
    pub fn is_pending(self) -> bool {
        matches!(self, Self::Waiting)
    }

    /// Op reached the ledger and the workspace file is reconciled.
    pub fn is_committed(self) -> bool {
        matches!(self, Self::Committed { .. })
    }

    /// Terminal failure surface for the user (rejected, or committed but the
    /// workspace writeback failed).
    pub fn is_failed(self) -> bool {
        matches!(self, Self::Rejected | Self::WritebackFailed { .. })
    }

    /// Ledger seq if the op reached the authority ledger (committed, possibly
    /// with a failed workspace writeback); `None` while waiting or if rejected.
    pub fn committed_seq(self) -> Option<u64> {
        match self {
            Self::Committed { seq } | Self::WritebackFailed { seq } => Some(seq),
            Self::Waiting | Self::Rejected => None,
        }
    }

    /// Apply a `ServerMessage::Ack` for this intent. Valid only from `Waiting`;
    /// a duplicate ack on an already-resolved intent is ignored (idempotent),
    /// so a late/replayed ack can never overwrite a recorded outcome.
    pub fn ack(self, seq: u64) -> Self {
        match self {
            Self::Waiting => Self::Committed { seq },
            resolved => resolved,
        }
    }

    /// Apply a `ServerMessage::EditRejected` for this intent. Valid only from
    /// `Waiting`; ignored once resolved.
    pub fn reject(self) -> Self {
        match self {
            Self::Waiting => Self::Rejected,
            resolved => resolved,
        }
    }

    /// Record that the workspace writeback failed after the op committed. Valid
    /// only from `Committed`; preserves the ledger seq. A writeback failure
    /// cannot apply to a waiting or rejected intent.
    pub fn writeback_failed(self) -> Self {
        match self {
            Self::Committed { seq } => Self::WritebackFailed { seq },
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waiting_is_pending() {
        let s = WriteConfirmation::waiting();
        assert!(s.is_pending());
        assert!(!s.is_committed());
        assert!(!s.is_failed());
        assert_eq!(s.committed_seq(), None);
    }

    #[test]
    fn ack_commits_with_seq() {
        let s = WriteConfirmation::waiting().ack(42);
        assert_eq!(s, WriteConfirmation::Committed { seq: 42 });
        assert!(s.is_committed());
        assert!(!s.is_pending());
        assert_eq!(s.committed_seq(), Some(42));
    }

    #[test]
    fn reject_from_waiting_is_failure() {
        let s = WriteConfirmation::waiting().reject();
        assert_eq!(s, WriteConfirmation::Rejected);
        assert!(s.is_failed());
        assert!(!s.is_committed());
        assert_eq!(s.committed_seq(), None);
    }

    #[test]
    fn writeback_failure_preserves_seq_and_is_failure() {
        let s = WriteConfirmation::waiting().ack(7).writeback_failed();
        assert_eq!(s, WriteConfirmation::WritebackFailed { seq: 7 });
        assert!(s.is_failed());
        // the op is still durable in the ledger
        assert_eq!(s.committed_seq(), Some(7));
    }

    #[test]
    fn ack_and_reject_are_idempotent_on_resolved_states() {
        // a duplicate ack keeps the original seq
        let committed = WriteConfirmation::waiting().ack(1);
        assert_eq!(committed.ack(2), WriteConfirmation::Committed { seq: 1 });
        // a late ack cannot resurrect a rejected intent
        let rejected = WriteConfirmation::waiting().reject();
        assert_eq!(rejected.ack(9), WriteConfirmation::Rejected);
        // a duplicate reject is a no-op once committed
        assert_eq!(committed.reject(), WriteConfirmation::Committed { seq: 1 });
    }

    #[test]
    fn writeback_failure_only_applies_after_commit() {
        // cannot fail writeback while still waiting
        assert_eq!(
            WriteConfirmation::waiting().writeback_failed(),
            WriteConfirmation::Waiting
        );
        // cannot fail writeback on a rejected intent
        assert_eq!(
            WriteConfirmation::Rejected.writeback_failed(),
            WriteConfirmation::Rejected
        );
    }
}
