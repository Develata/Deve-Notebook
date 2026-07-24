//! plan_ref:
//!   - 06_backup#remote-import-session-contract
//!   - 07_network#remote-import-wire-contract
//!   - 09_web_thin_client_ledger#remote-import-client-contract
//!
//! Typed Remote Import thin client. It owns only request correlation and
//! browser projection state; provider I/O, blockers, diffs and Apply authority
//! remain backend-owned.

use crate::api::WsService;
use crate::runtime::domain::{PendingBranchSwitch, PendingRepoSwitch};
use deve_core::models::PeerId;
use deve_core::protocol::{
    ClientMessage, RemoteImportCandidateRevision, RemoteImportEntryId, RemoteImportPageCursor,
    RemoteImportRequest, RemoteImportRequestContext, RemoteImportResponse, RemoteImportSessionId,
    RemoteProjectionProvider,
};
use leptos::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

mod accept;
mod model;
mod request;
mod scope;

pub use model::{
    RemoteImportAvailability, RemoteImportDiffProjection, RemoteImportPendingState,
    RemoteImportProjection, RemoteImportSelection,
};
use request::{
    PendingKind, PendingLane, PendingRequest, RemoteImportScope, SelectionIdentity, SelectionState,
};

#[derive(Clone)]
pub struct RemoteImportClient {
    ws: WsService,
    current_repo_id: ReadSignal<Option<String>>,
    active_branch: ReadSignal<Option<PeerId>>,
    current_scope_nonce: ReadSignal<u64>,
    pending_branch_switch: ReadSignal<Option<PendingBranchSwitch>>,
    pending_repo_switch: ReadSignal<Option<PendingRepoSwitch>>,
    pending: Arc<Mutex<HashMap<Uuid, PendingRequest>>>,
    selection: Arc<Mutex<SelectionState>>,
    projection: RwSignal<RemoteImportProjection>,
    projection_read: ReadSignal<RemoteImportProjection>,
    projected_scope: Arc<Mutex<Option<RemoteImportScope>>>,
}

impl RemoteImportClient {
    #[allow(clippy::too_many_arguments)] // Exact scope inputs stay explicit at the client boundary.
    pub fn new(
        ws: WsService,
        current_repo_id: ReadSignal<Option<String>>,
        active_branch: ReadSignal<Option<PeerId>>,
        current_scope_nonce: ReadSignal<u64>,
        pending_branch_switch: ReadSignal<Option<PendingBranchSwitch>>,
        pending_repo_switch: ReadSignal<Option<PendingRepoSwitch>>,
    ) -> Self {
        let projection = RwSignal::new(RemoteImportProjection::default());
        let projection_read = projection.read_only();
        Self {
            ws,
            current_repo_id,
            active_branch,
            current_scope_nonce,
            pending_branch_switch,
            pending_repo_switch,
            pending: Arc::new(Mutex::new(HashMap::new())),
            selection: Arc::new(Mutex::new(SelectionState::default())),
            projection,
            projection_read,
            projected_scope: Arc::new(Mutex::new(None)),
        }
    }

    pub fn projection(&self) -> ReadSignal<RemoteImportProjection> {
        self.projection_read
    }

    pub fn prepare(&self, provider: RemoteProjectionProvider) -> Option<Uuid> {
        self.send(PendingKind::Prepare, move |context| {
            RemoteImportRequest::Prepare { context, provider }
        })
    }

    pub fn list(&self) -> Option<Uuid> {
        self.send(PendingKind::List, |context| RemoteImportRequest::List {
            context,
        })
    }

    pub fn show(
        &self,
        session_id: RemoteImportSessionId,
        revision: Option<RemoteImportCandidateRevision>,
    ) -> Option<Uuid> {
        self.synchronize_current_scope()?;
        self.begin_selection(session_id, revision);
        self.send(
            PendingKind::Show {
                session_id,
                revision,
            },
            move |context| RemoteImportRequest::Show {
                context,
                session_id,
                revision,
            },
        )
    }

    pub fn first_page(
        &self,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
    ) -> Option<Uuid> {
        self.page(session_id, revision, None)
    }

    pub fn next_page(&self) -> Option<Uuid> {
        let projection = self.projection.get_untracked();
        let selected = projection.selected_session?;
        self.page(
            selected.session_id,
            selected.revision?,
            projection.next_cursor,
        )
    }

    pub fn diff(
        &self,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
        entry_id: RemoteImportEntryId,
    ) -> Option<Uuid> {
        self.send(
            PendingKind::Diff {
                session_id,
                revision,
                entry_id: entry_id.clone(),
            },
            move |context| RemoteImportRequest::Diff {
                context,
                session_id,
                revision,
                entry_id,
            },
        )
    }

    pub fn refresh(
        &self,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
    ) -> Option<Uuid> {
        self.send(
            PendingKind::Refresh {
                session_id,
                revision,
            },
            move |context| RemoteImportRequest::Refresh {
                context,
                session_id,
                revision,
            },
        )
    }

    pub fn apply(
        &self,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
    ) -> Option<Uuid> {
        self.send(
            PendingKind::Apply {
                session_id,
                revision,
            },
            move |context| RemoteImportRequest::Apply {
                context,
                session_id,
                revision,
            },
        )
    }

    pub fn discard(
        &self,
        session_id: RemoteImportSessionId,
        revision: Option<RemoteImportCandidateRevision>,
    ) -> Option<Uuid> {
        self.send(
            PendingKind::Discard {
                session_id,
                revision,
            },
            move |context| RemoteImportRequest::Discard {
                context,
                session_id,
                revision,
            },
        )
    }

    pub fn clear_diff(&self) {
        self.projection.update(|projection| projection.diff = None);
    }

    fn page(
        &self,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
        cursor: Option<RemoteImportPageCursor>,
    ) -> Option<Uuid> {
        self.send(
            PendingKind::Page {
                session_id,
                revision,
                cursor: cursor.clone(),
            },
            move |context| RemoteImportRequest::Page {
                context,
                session_id,
                revision,
                cursor,
                limit: deve_core::protocol::REMOTE_IMPORT_DEFAULT_PAGE_SIZE,
            },
        )
    }

    fn send(
        &self,
        kind: PendingKind,
        request: impl FnOnce(RemoteImportRequestContext) -> RemoteImportRequest,
    ) -> Option<Uuid> {
        let scope = self.synchronize_current_scope()?;
        let selection_generation = if let Some(expected) = kind.selection() {
            let selected = self
                .selection
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if selected.identity != Some(expected) {
                return None;
            }
            Some(selected.generation)
        } else {
            None
        };
        let request_id = Uuid::new_v4();
        let context = RemoteImportRequestContext {
            request_id,
            repo_id: scope.repo_id,
            branch: scope.branch.clone(),
            scope_nonce: scope.scope_nonce,
        };
        {
            let mut pending = self
                .pending
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let lane = kind.lane();
            if lane == PendingLane::Diff {
                pending.retain(|_, request| {
                    request.scope != scope
                        || request.selection_generation != selection_generation
                        || request.kind.lane() != PendingLane::Diff
                });
            } else if pending.values().any(|request| {
                request.scope == scope
                    && request.selection_generation == selection_generation
                    && request.kind.lane() == lane
            }) {
                return None;
            }
            pending.insert(
                request_id,
                PendingRequest {
                    scope: scope.clone(),
                    selection_generation,
                    kind,
                },
            );
        }
        self.publish_pending_state();
        self.ws.send(ClientMessage::RemoteImport(request(context)));
        Some(request_id)
    }

    fn begin_selection(
        &self,
        session_id: RemoteImportSessionId,
        revision: Option<RemoteImportCandidateRevision>,
    ) {
        let identity = SelectionIdentity {
            session_id,
            revision,
        };
        let generation = {
            let mut selection = self
                .selection
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            selection.generation = selection.generation.saturating_add(1);
            selection.identity = Some(identity);
            selection.generation
        };
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|_, request| request.selection_generation.is_none());
        self.projection.update(|projection| {
            projection.begin_selection(RemoteImportSelection {
                session_id,
                revision,
            });
        });
        debug_assert!(generation > 0);
        self.publish_pending_state();
    }

    fn install_backend_selection(
        &self,
        session_id: RemoteImportSessionId,
        revision: Option<RemoteImportCandidateRevision>,
    ) {
        self.begin_selection(session_id, revision);
    }

    fn selection_accepts(&self, request: &PendingRequest) -> bool {
        let Some(generation) = request.selection_generation else {
            return true;
        };
        let selection = self
            .selection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        selection.generation == generation && selection.identity == request.kind.selection()
    }

    fn publish_pending_state(&self) {
        let mut state = RemoteImportPendingState::default();
        for request in self
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .values()
        {
            request.kind.mark_pending(&mut state);
        }
        self.projection
            .update(|projection| projection.pending = state);
    }

    pub(crate) fn accept(&self, response: RemoteImportResponse) -> bool {
        accept::accept(self, response)
    }
}

#[cfg(test)]
#[path = "remote_import_client/tests.rs"]
mod tests;
