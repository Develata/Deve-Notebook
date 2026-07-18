//! plan_ref:
//!   - 07_network#remote-import-wire-contract
//!
//! Shared safe projection from host-only Remote Import domain views to public
//! wire DTOs. No locator, path, digest, credential, or raw error can cross it.

use deve_core::protocol as wire;
use deve_core::remote_import as domain;

pub(crate) fn session_id(value: domain::RemoteImportSessionId) -> wire::RemoteImportSessionId {
    wire::RemoteImportSessionId::new(value.as_uuid())
}

pub(crate) fn revision(
    value: domain::RemoteImportCandidateRevision,
) -> wire::RemoteImportCandidateRevision {
    wire::RemoteImportCandidateRevision::new(value.get())
}

pub(crate) fn entry_id(value: domain::RemoteImportEntryId) -> wire::RemoteImportEntryId {
    wire::RemoteImportEntryId::new(value.as_str())
}

pub(crate) fn state(value: domain::RemoteImportState) -> wire::RemoteImportState {
    match value {
        domain::RemoteImportState::Preparing => wire::RemoteImportState::Preparing,
        domain::RemoteImportState::Ready => wire::RemoteImportState::Ready,
        domain::RemoteImportState::Stale => wire::RemoteImportState::Stale,
        domain::RemoteImportState::Failed => wire::RemoteImportState::Failed,
        domain::RemoteImportState::Applied => wire::RemoteImportState::Applied,
        domain::RemoteImportState::Discarded => wire::RemoteImportState::Discarded,
    }
}

pub(crate) fn change_kind(value: domain::RemoteImportChangeKind) -> wire::RemoteImportChangeKind {
    match value {
        domain::RemoteImportChangeKind::Added => wire::RemoteImportChangeKind::Added,
        domain::RemoteImportChangeKind::Modified => wire::RemoteImportChangeKind::Modified,
        domain::RemoteImportChangeKind::Unchanged => wire::RemoteImportChangeKind::Unchanged,
    }
}

pub(crate) fn blocker(value: domain::RemoteImportBlocker) -> wire::RemoteImportBlocker {
    match value {
        domain::RemoteImportBlocker::LedgerHeadDrift => wire::RemoteImportBlocker::LedgerHeadDrift,
        domain::RemoteImportBlocker::IgnoreSnapshotDrift => {
            wire::RemoteImportBlocker::IgnoreSnapshotDrift
        }
        domain::RemoteImportBlocker::LocatorBindingDrift => {
            wire::RemoteImportBlocker::LocatorBindingDrift
        }
        domain::RemoteImportBlocker::PendingOverlap => wire::RemoteImportBlocker::PendingOverlap,
        domain::RemoteImportBlocker::StagedOverlap => wire::RemoteImportBlocker::StagedOverlap,
        domain::RemoteImportBlocker::ArtifactTamper => wire::RemoteImportBlocker::ArtifactTamper,
        domain::RemoteImportBlocker::RepoMembershipMismatch => {
            wire::RemoteImportBlocker::RepoMembershipMismatch
        }
    }
}

fn projection_outcome(
    value: domain::RemoteImportProjectionOutcome,
) -> wire::RemoteImportProjectionOutcome {
    match value {
        domain::RemoteImportProjectionOutcome::Pending => {
            wire::RemoteImportProjectionOutcome::Pending
        }
        domain::RemoteImportProjectionOutcome::Written => {
            wire::RemoteImportProjectionOutcome::Written
        }
        domain::RemoteImportProjectionOutcome::Degraded => {
            wire::RemoteImportProjectionOutcome::Degraded
        }
    }
}

pub(crate) fn session(value: domain::RemoteImportSessionView) -> wire::RemoteImportSessionView {
    wire::RemoteImportSessionView {
        session_id: session_id(value.session_id),
        state: state(value.state),
        revision: value.revision.map(revision),
        entry_count: value.entry_count,
        blockers: value.blockers.into_iter().map(blocker).collect(),
        cleanup_pending: value.cleanup_pending,
        projection_outcome: value.projection_outcome.map(projection_outcome),
    }
}

pub(crate) fn page(value: domain::RemoteImportCandidatePage) -> wire::RemoteImportCandidatePage {
    wire::RemoteImportCandidatePage {
        session: session(value.session),
        entries: value
            .entries
            .into_iter()
            .map(|entry| wire::RemoteImportCandidateView {
                entry_id: entry_id(entry.entry_id),
                display_label: entry.display_label,
                change_kind: change_kind(entry.change_kind),
                blockers: entry.blockers.into_iter().map(blocker).collect(),
            })
            .collect(),
        next_cursor: value
            .next_cursor
            .map(|cursor| wire::RemoteImportPageCursor::new(cursor.as_str())),
    }
}

pub(crate) fn receipt(value: domain::RemoteImportApplyView) -> wire::RemoteImportApplyReceipt {
    wire::RemoteImportApplyReceipt {
        request_id: value.request_id,
        session_id: session_id(value.session_id),
        revision: revision(value.revision),
        projection_outcome: projection_outcome(value.projection_outcome),
    }
}
