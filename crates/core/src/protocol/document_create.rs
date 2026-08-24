//! plan_ref:
//!   - 07_network#projection-recovery-contract
//!   - 09_web_thin_client_ledger#document-create-intent
//!
//! Typed, scope-bound Document Create request/response contract.

use super::{ScopeNonce, ServerError};
use crate::models::{DocId, NodeId, PeerId, RepoId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentCreateRequest {
    pub proposed_node_id: NodeId,
    pub repo_id: RepoId,
    pub branch: Option<PeerId>,
    pub scope_nonce: ScopeNonce,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentCreateResponseContext {
    pub proposed_node_id: NodeId,
    pub repo_id: RepoId,
    pub branch: Option<PeerId>,
    pub scope_nonce: ScopeNonce,
}

impl From<&DocumentCreateRequest> for DocumentCreateResponseContext {
    fn from(request: &DocumentCreateRequest) -> Self {
        Self {
            proposed_node_id: request.proposed_node_id,
            repo_id: request.repo_id,
            branch: request.branch.clone(),
            scope_nonce: request.scope_nonce,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentCreateProjectionOutcome {
    Written,
    RecoveryRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentCreateResponse {
    Created {
        context: DocumentCreateResponseContext,
        node_id: NodeId,
        doc_id: Option<DocId>,
        path: String,
        projection_outcome: DocumentCreateProjectionOutcome,
    },
    Rejected {
        context: DocumentCreateResponseContext,
        error: ServerError,
    },
}

impl DocumentCreateResponse {
    pub fn context(&self) -> &DocumentCreateResponseContext {
        match self {
            Self::Created { context, .. } | Self::Rejected { context, .. } => context,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_create_v6_wire_roundtrips_client_identity_and_typed_result() {
        let request = DocumentCreateRequest {
            proposed_node_id: NodeId::new(),
            repo_id: RepoId::new_v4(),
            branch: None,
            scope_nonce: ScopeNonce::new(7),
            path: "notes/new.md".into(),
        };
        let encoded = postcard::to_allocvec(&request).expect("encode request");
        let decoded: DocumentCreateRequest =
            postcard::from_bytes(&encoded).expect("decode request");
        assert_eq!(decoded, request);

        let response = DocumentCreateResponse::Created {
            context: (&request).into(),
            node_id: request.proposed_node_id,
            doc_id: Some(DocId(request.proposed_node_id.0)),
            path: "notes/new.md".into(),
            projection_outcome: DocumentCreateProjectionOutcome::Written,
        };
        let encoded = postcard::to_allocvec(&response).expect("encode response");
        let decoded: DocumentCreateResponse =
            postcard::from_bytes(&encoded).expect("decode response");
        assert_eq!(decoded, response);
    }
}
