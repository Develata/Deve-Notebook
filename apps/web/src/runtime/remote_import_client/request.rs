//! plan_ref:
//!   - 06_backup#remote-import-session-contract
//!   - 07_network#remote-import-wire-contract
//!   - 09_web_thin_client_ledger#remote-import-client-contract
//!
//! Request correlation, selection generation, and single-flight lanes.

use super::model::RemoteImportPendingState;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{
    RemoteImportCandidateRevision, RemoteImportEntryId, RemoteImportPageCursor,
    RemoteImportSessionId, ScopeNonce,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RemoteImportScope {
    pub connection_epoch: u64,
    pub repo_id: RepoId,
    pub branch: Option<PeerId>,
    pub scope_nonce: ScopeNonce,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SelectionIdentity {
    pub session_id: RemoteImportSessionId,
    pub revision: Option<RemoteImportCandidateRevision>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct SelectionState {
    pub generation: u64,
    pub identity: Option<SelectionIdentity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PendingLane {
    Prepare,
    List,
    Show,
    Page,
    Diff,
    Refresh,
    Apply,
    Discard,
}

#[derive(Clone, Debug)]
pub(super) enum PendingKind {
    Prepare,
    List,
    Show {
        session_id: RemoteImportSessionId,
        revision: Option<RemoteImportCandidateRevision>,
    },
    Page {
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
        cursor: Option<RemoteImportPageCursor>,
    },
    Diff {
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
        entry_id: RemoteImportEntryId,
    },
    Refresh {
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
    },
    Apply {
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
    },
    Discard {
        session_id: RemoteImportSessionId,
        revision: Option<RemoteImportCandidateRevision>,
    },
}

impl PendingKind {
    pub fn lane(&self) -> PendingLane {
        match self {
            Self::Prepare => PendingLane::Prepare,
            Self::List => PendingLane::List,
            Self::Show { .. } => PendingLane::Show,
            Self::Page { .. } => PendingLane::Page,
            Self::Diff { .. } => PendingLane::Diff,
            Self::Refresh { .. } => PendingLane::Refresh,
            Self::Apply { .. } => PendingLane::Apply,
            Self::Discard { .. } => PendingLane::Discard,
        }
    }

    pub fn selection(&self) -> Option<SelectionIdentity> {
        match self {
            Self::Prepare | Self::List => None,
            Self::Show {
                session_id,
                revision,
            }
            | Self::Discard {
                session_id,
                revision,
            } => Some(SelectionIdentity {
                session_id: *session_id,
                revision: *revision,
            }),
            Self::Page {
                session_id,
                revision,
                ..
            }
            | Self::Diff {
                session_id,
                revision,
                ..
            }
            | Self::Refresh {
                session_id,
                revision,
            }
            | Self::Apply {
                session_id,
                revision,
            } => Some(SelectionIdentity {
                session_id: *session_id,
                revision: Some(*revision),
            }),
        }
    }

    pub fn mark_pending(&self, state: &mut RemoteImportPendingState) {
        match self.lane() {
            PendingLane::Prepare => state.prepare = true,
            PendingLane::List => state.list = true,
            PendingLane::Show => state.show = true,
            PendingLane::Page => state.page = true,
            PendingLane::Diff => state.diff = true,
            PendingLane::Refresh => state.refresh = true,
            PendingLane::Apply => state.apply = true,
            PendingLane::Discard => state.discard = true,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct PendingRequest {
    pub scope: RemoteImportScope,
    pub selection_generation: Option<u64>,
    pub kind: PendingKind,
}
