//! plan_ref:
//!   - 07_network#projection-recovery-contract
//!   - 09_web_thin_client_ledger#projection-recovery-coordinator
//!
use deve_core::models::{DocId, PeerId, RepoId};
use deve_core::protocol::{
    DocumentRecoveryScope, ProjectionRecoveryPlan, ProjectionRecoveryRequired,
};

#[derive(Clone, Debug)]
pub struct ProjectionRecoveryScope {
    pub repo_id: Option<RepoId>,
    pub branch: Option<PeerId>,
    pub scope_nonce: u64,
    pub current_doc: Option<DocId>,
    pub scope_switch_pending: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectionRecoveryDecision {
    pub plan: ProjectionRecoveryPlan,
    pub current_document_affected: bool,
}

impl ProjectionRecoveryDecision {
    pub fn requires_refresh(&self) -> bool {
        self.plan.refresh_doc_list
            || self.plan.refresh_source_control
            || self.plan.refresh_external_changes
    }
}

pub fn evaluate_recovery(
    required: &ProjectionRecoveryRequired,
    scope: &ProjectionRecoveryScope,
) -> Option<ProjectionRecoveryDecision> {
    if scope.scope_switch_pending
        || scope.repo_id != Some(required.repo_id)
        || scope.branch != required.branch
        || required.scope_nonce != Some(scope.scope_nonce)
    {
        return None;
    }
    let current_document_affected = match &required.plan.documents {
        DocumentRecoveryScope::None => false,
        DocumentRecoveryScope::Exact(docs) => {
            scope.current_doc.is_some_and(|doc| docs.contains(&doc))
        }
        DocumentRecoveryScope::CurrentDocument => scope.current_doc.is_some(),
    };
    Some(ProjectionRecoveryDecision {
        plan: required.plan.clone(),
        current_document_affected,
    })
}
