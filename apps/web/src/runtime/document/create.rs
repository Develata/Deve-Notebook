//! plan_ref:
//!   - 09_web_thin_client_ledger#document-create-intent
//!   - 07_network#projection-recovery-contract
//!
//! Pure pending-state contract for one idempotent Document Create intent.

use deve_core::models::{DocId, NodeId, RepoId};
use deve_core::protocol::{DocumentCreateRequest, DocumentCreateResponse, ScopeNonce, ServerError};

#[derive(Clone, Debug, PartialEq, Eq)]
enum CreateConfirmation {
    Waiting,
    Created { doc_id: Option<DocId>, path: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingDocumentCreate {
    proposed_node_id: NodeId,
    repo_id: RepoId,
    scope_nonce: u64,
    path: String,
    select_when_projected: bool,
    confirmation: CreateConfirmation,
    rebound_after_internal_reconnect: bool,
    replayed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CreateResponseDisposition {
    Ignored,
    WaitingForProjection,
    CompletedWithoutDocument,
    Rejected(ServerError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectedCreate {
    pub doc_id: DocId,
    pub select: bool,
}

impl PendingDocumentCreate {
    pub fn new(repo_id: RepoId, scope_nonce: u64, path: String, select: bool) -> Self {
        Self {
            proposed_node_id: NodeId::new(),
            repo_id,
            scope_nonce,
            path,
            select_when_projected: select,
            confirmation: CreateConfirmation::Waiting,
            rebound_after_internal_reconnect: false,
            replayed: false,
        }
    }

    pub fn request(&self) -> DocumentCreateRequest {
        DocumentCreateRequest {
            proposed_node_id: self.proposed_node_id,
            repo_id: self.repo_id,
            branch: None,
            scope_nonce: ScopeNonce::new(self.scope_nonce),
            path: self.path.clone(),
        }
    }

    #[cfg(test)]
    pub fn proposed_node_id(&self) -> NodeId {
        self.proposed_node_id
    }

    pub fn accept_response(
        &mut self,
        response: &DocumentCreateResponse,
    ) -> CreateResponseDisposition {
        let context = response.context();
        if context.proposed_node_id != self.proposed_node_id
            || context.repo_id != self.repo_id
            || context.branch.is_some()
            || context.scope_nonce.get() != self.scope_nonce
        {
            return CreateResponseDisposition::Ignored;
        }
        match response {
            DocumentCreateResponse::Created {
                node_id,
                doc_id,
                path,
                ..
            } if *node_id == self.proposed_node_id
                && doc_id.is_none_or(|doc_id| NodeId::from_doc_id(doc_id) == *node_id) =>
            {
                self.confirmation = CreateConfirmation::Created {
                    doc_id: *doc_id,
                    path: path.clone(),
                };
                if doc_id.is_some() {
                    CreateResponseDisposition::WaitingForProjection
                } else {
                    CreateResponseDisposition::CompletedWithoutDocument
                }
            }
            DocumentCreateResponse::Created { .. } => CreateResponseDisposition::Ignored,
            DocumentCreateResponse::Rejected { error, .. } => {
                CreateResponseDisposition::Rejected(error.clone())
            }
        }
    }

    pub fn observe_docs(&self, docs: &[(DocId, String)]) -> Option<ProjectedCreate> {
        let CreateConfirmation::Created {
            doc_id: Some(expected_doc_id),
            ref path,
        } = self.confirmation
        else {
            return None;
        };
        docs.iter()
            .any(|(doc_id, projected_path)| *doc_id == expected_doc_id && projected_path == path)
            .then_some(ProjectedCreate {
                doc_id: expected_doc_id,
                select: self.select_when_projected,
            })
    }

    pub fn rebind_internal_reconnect(
        &mut self,
        repo_id: RepoId,
        previous_scope_nonce: u64,
        next_scope_nonce: u64,
    ) -> bool {
        if self.repo_id != repo_id
            || self.scope_nonce != previous_scope_nonce
            || !matches!(self.confirmation, CreateConfirmation::Waiting)
            || self.replayed
        {
            return false;
        }
        self.scope_nonce = next_scope_nonce;
        self.rebound_after_internal_reconnect = true;
        true
    }

    pub fn take_replay_for_write_ready(
        &mut self,
        repo_id: RepoId,
        scope_nonce: u64,
    ) -> Option<DocumentCreateRequest> {
        if self.repo_id != repo_id
            || self.scope_nonce != scope_nonce
            || !self.rebound_after_internal_reconnect
            || self.replayed
            || !matches!(self.confirmation, CreateConfirmation::Waiting)
        {
            return None;
        }
        self.replayed = true;
        Some(self.request())
    }
}

#[cfg(test)]
mod tests;
