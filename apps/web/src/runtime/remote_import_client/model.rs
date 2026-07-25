//! plan_ref:
//!   - 06_backup#remote-import-session-contract
//!   - 09_web_thin_client_ledger#remote-import-client-contract
//!
//! Browser projection model for the typed Remote Import client.

use deve_core::protocol::{
    RemoteImportApplyReceipt, RemoteImportCandidatePage, RemoteImportCandidateRevision,
    RemoteImportCandidateView, RemoteImportPageCursor, RemoteImportProjectionOutcome,
    RemoteImportSessionId, RemoteImportSessionView, ServerErrorCode,
};
use deve_core::source_control::diff_projection::DiffProjection;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RemoteImportAvailability {
    #[default]
    Offline,
    NoRepo,
    ScopeTransitioning,
    Ready {
        workspace_ingestion_blocked: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemoteImportSelection {
    pub session_id: RemoteImportSessionId,
    pub revision: Option<RemoteImportCandidateRevision>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RemoteImportPendingState {
    pub prepare: bool,
    pub list: bool,
    pub show: bool,
    pub page: bool,
    pub diff: bool,
    pub refresh: bool,
    pub apply: bool,
    pub discard: bool,
}

impl RemoteImportPendingState {
    pub fn any(self) -> bool {
        self.prepare
            || self.list
            || self.show
            || self.page
            || self.diff
            || self.refresh
            || self.apply
            || self.discard
    }
}

#[derive(Clone, Debug)]
pub struct RemoteImportDiffProjection {
    pub session_id: RemoteImportSessionId,
    pub revision: RemoteImportCandidateRevision,
    pub entry_id: deve_core::protocol::RemoteImportEntryId,
    pub display_label: String,
    pub change_kind: deve_core::protocol::RemoteImportChangeKind,
    pub blockers: Vec<deve_core::protocol::RemoteImportBlocker>,
    pub projection: Arc<DiffProjection>,
}

#[derive(Clone, Debug, Default)]
pub struct RemoteImportProjection {
    pub availability: RemoteImportAvailability,
    pub pending: RemoteImportPendingState,
    pub sessions: Vec<RemoteImportSessionView>,
    pub selection: Option<RemoteImportSelection>,
    pub selected_session: Option<RemoteImportSessionView>,
    pub entries: Vec<RemoteImportCandidateView>,
    pub next_cursor: Option<RemoteImportPageCursor>,
    pub diff: Option<RemoteImportDiffProjection>,
    pub last_apply: Option<RemoteImportApplyReceipt>,
    pub error: Option<ServerErrorCode>,
}

impl RemoteImportProjection {
    pub(super) fn begin_selection(&mut self, selection: RemoteImportSelection) {
        self.selection = Some(selection);
        self.selected_session = self
            .sessions
            .iter()
            .find(|session| {
                session.session_id == selection.session_id && session.revision == selection.revision
            })
            .cloned();
        self.entries.clear();
        self.next_cursor = None;
        self.diff = None;
        self.error = None;
    }

    pub(super) fn install_session(&mut self, session: RemoteImportSessionView) {
        if let Some(existing) = self
            .sessions
            .iter_mut()
            .find(|existing| existing.session_id == session.session_id)
        {
            *existing = session.clone();
        } else {
            self.sessions.push(session.clone());
        }
        if self.selection.is_some_and(|selection| {
            selection.session_id == session.session_id && selection.revision == session.revision
        }) {
            self.selected_session = Some(session);
        }
        self.error = None;
    }

    pub(super) fn install_page(&mut self, page: RemoteImportCandidatePage, append: bool) {
        self.install_session(page.session);
        if append {
            self.entries.extend(page.entries);
        } else {
            self.entries = page.entries;
        }
        self.next_cursor = page.next_cursor;
    }

    pub fn apply_receipt_for(
        &self,
        session_id: RemoteImportSessionId,
        revision: Option<RemoteImportCandidateRevision>,
    ) -> Option<&RemoteImportApplyReceipt> {
        self.last_apply.as_ref().filter(|receipt| {
            receipt.session_id == session_id && Some(receipt.revision) == revision
        })
    }

    pub fn apply_outcome_for(
        &self,
        session_id: RemoteImportSessionId,
        revision: Option<RemoteImportCandidateRevision>,
    ) -> Option<RemoteImportProjectionOutcome> {
        self.apply_receipt_for(session_id, revision)
            .map(|receipt| receipt.projection_outcome)
            .or_else(|| {
                self.sessions
                    .iter()
                    .find(|session| {
                        session.session_id == session_id && session.revision == revision
                    })
                    .and_then(|session| session.projection_outcome)
            })
    }

    pub fn selected_apply_completed(&self) -> bool {
        self.selection.is_some_and(|selection| {
            self.apply_receipt_for(selection.session_id, selection.revision)
                .is_some()
        })
    }
}
