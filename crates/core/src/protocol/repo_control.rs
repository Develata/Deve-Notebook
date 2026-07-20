//! plan_ref:
//!   - 07_network#repo-control-wire-contract
//!   - 09_web_thin_client_ledger#repo-control-client-contract

use super::{ScopeNonce, ServerError, SwitchNonce};
use crate::models::RepoId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepoReadiness {
    Mounted,
    Readonly,
    Transitioning,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoAliasBinding {
    pub repo_id: RepoId,
    pub display_alias: String,
    pub alias_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum RepoLifecycleIntent {
    Create {
        initial_alias: String,
        current_scope_nonce: ScopeNonce,
        switch_nonce: SwitchNonce,
    },
    Remove {
        repo_id: RepoId,
        current_scope_nonce: ScopeNonce,
        switch_nonce: SwitchNonce,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoControlRequest {
    SetAlias {
        request_id: Uuid,
        repo_id: RepoId,
        alias: String,
        expected_alias_revision: u64,
    },
    SubmitLifecycle {
        request_id: Uuid,
        lifecycle_intent: RepoLifecycleIntent,
    },
    GetLifecycle {
        request_id: Uuid,
    },
}

impl RepoControlRequest {
    pub const fn request_id(&self) -> Uuid {
        match self {
            Self::SetAlias { request_id, .. }
            | Self::SubmitLifecycle { request_id, .. }
            | Self::GetLifecycle { request_id } => *request_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepoLifecycleOperation {
    Create,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepoLifecycleState {
    Accepted,
    Running,
    Recovering,
    Terminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepoLifecycleOutcome {
    Succeeded,
    NotCommitted,
    CommittedPartial,
    RepairRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoControlResponse {
    AliasSet {
        request_id: Uuid,
        binding: RepoAliasBinding,
    },
    LifecycleAccepted {
        request_id: Uuid,
        job_id: Uuid,
        target_repo_id: RepoId,
    },
    LifecycleStatus {
        request_id: Uuid,
        job_id: Uuid,
        target_repo_id: RepoId,
        operation: RepoLifecycleOperation,
        state: RepoLifecycleState,
        outcome: Option<RepoLifecycleOutcome>,
        publication_pending: bool,
    },
    Error {
        request_id: Uuid,
        error: ServerError,
    },
}
