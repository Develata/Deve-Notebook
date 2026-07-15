//! plan_ref:
//!   - 07_network#projection-recovery-contract
//!   - 09_web_thin_client_ledger#projection-recovery-coordinator
//!
//! Backend-owned projection recovery contract shared by every wire surface.

use crate::models::{DocId, PeerId, RepoId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionRecoveryRequired {
    pub repo_id: RepoId,
    pub branch: Option<PeerId>,
    pub scope_nonce: Option<u64>,
    pub cause: ProjectionRecoveryCause,
    pub plan: ProjectionRecoveryPlan,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectionRecoveryCause {
    ExternalApply,
    DocumentMutation,
    SourceControlCommit,
    Merge,
    PluginMutation,
    BroadcastGap { skipped: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionRecoveryPlan {
    pub documents: DocumentRecoveryScope,
    pub refresh_doc_list: bool,
    pub refresh_source_control: bool,
    pub refresh_external_changes: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentRecoveryScope {
    None,
    Exact(Vec<DocId>),
    CurrentDocument,
}

impl ProjectionRecoveryPlan {
    pub fn external_apply(affected_docs: Vec<DocId>) -> Self {
        Self {
            documents: DocumentRecoveryScope::Exact(affected_docs),
            refresh_doc_list: true,
            refresh_source_control: true,
            refresh_external_changes: true,
        }
    }

    pub fn broadcast_gap() -> Self {
        Self {
            documents: DocumentRecoveryScope::CurrentDocument,
            refresh_doc_list: true,
            refresh_source_control: true,
            refresh_external_changes: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_apply_plan_keeps_exact_document_scope() {
        let doc_id = DocId::new();
        let plan = ProjectionRecoveryPlan::external_apply(vec![doc_id]);

        assert_eq!(plan.documents, DocumentRecoveryScope::Exact(vec![doc_id]));
        assert!(plan.refresh_doc_list);
        assert!(plan.refresh_source_control);
        assert!(plan.refresh_external_changes);
    }
}
