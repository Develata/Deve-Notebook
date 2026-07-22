//! plan_ref:
//!   - 07_network#repo-control-wire-contract
//!   - 09_web_thin_client_ledger#repo-control-client-contract

use super::{ScopeNonce, ServerError, SwitchNonce};
use crate::models::RepoId;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
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

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RemovalConfirmationToken(String);

impl RemovalConfirmationToken {
    pub fn from_backend(value: String) -> Option<Self> {
        is_hex_256(&value).then_some(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RemovalConfirmationToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RemovalConfirmationToken([redacted])")
    }
}

impl<'de> Deserialize<'de> for RemovalConfirmationToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_backend(value)
            .ok_or_else(|| serde::de::Error::custom("invalid 256-bit removal confirmation token"))
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct OpaqueFallbackBinding(String);

impl OpaqueFallbackBinding {
    pub fn from_backend(value: String) -> Option<Self> {
        is_hex_256(&value).then_some(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for OpaqueFallbackBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OpaqueFallbackBinding([redacted])")
    }
}

impl<'de> Deserialize<'de> for OpaqueFallbackBinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_backend(value)
            .ok_or_else(|| serde::de::Error::custom("invalid 256-bit opaque fallback binding"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalRepoRemovalDeletedCategory {
    LocalLedgerAuthority,
    DeveRuntimeMetadata,
    ProjectionLocator,
    HostAlias,
    RemoteImportCaptures,
    CatalogMembership,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalRepoRemovalPreservedCategory {
    WorkspaceContent,
    GitMetadata,
    RemoteShadows,
    HostIdentityAndConfiguration,
    OperatorRecoveryInputs,
    AuthorityLockIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalRepoRemovalWarning {
    LedgerHistoryHasNoSupportedRestore,
    NoFallbackSelected,
    SelectedFallbackUnavailable,
    RemoteImportCaptureWillBeDiscarded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalRepoRemovalBlocker {
    ProjectionFault,
    WorkspaceIngestionUnavailable,
    AuthorityBusy,
    RepositoryIdentityAmbiguous,
    WorkspaceIdentityUnverified,
    RecoveryInputOverlap,
    RemoteImportApplyInFlight,
    RemoteImportProjectionPending,
    RemoteImportProjectionDegraded,
    RepairRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalRepoRemovalPreview {
    pub deleted: Vec<LocalRepoRemovalDeletedCategory>,
    pub preserved: Vec<LocalRepoRemovalPreservedCategory>,
    pub warnings: Vec<LocalRepoRemovalWarning>,
    pub blockers: Vec<LocalRepoRemovalBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoRemovalFinalScope {
    RepoBound {
        repo_id: RepoId,
        scope_nonce: ScopeNonce,
    },
    NoScope {
        scope_nonce: ScopeNonce,
    },
}

impl RepoRemovalFinalScope {
    pub const fn scope_nonce(&self) -> ScopeNonce {
        match self {
            Self::RepoBound { scope_nonce, .. } | Self::NoScope { scope_nonce } => *scope_nonce,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoLifecycleIntent {
    Create {
        initial_alias: String,
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
    PrepareLocalRepoRemoval {
        request_id: Uuid,
        repo_id: RepoId,
        current_scope_nonce: ScopeNonce,
        fallback_repo_id: Option<RepoId>,
    },
    ExecuteLocalRepoRemoval {
        request_id: Uuid,
        preparation_id: Uuid,
        confirmation_token: RemovalConfirmationToken,
        fallback_binding: Option<OpaqueFallbackBinding>,
        current_scope_nonce: ScopeNonce,
        switch_nonce: SwitchNonce,
    },
}

impl RepoControlRequest {
    pub const fn request_id(&self) -> Uuid {
        match self {
            Self::SetAlias { request_id, .. }
            | Self::SubmitLifecycle { request_id, .. }
            | Self::GetLifecycle { request_id }
            | Self::PrepareLocalRepoRemoval { request_id, .. }
            | Self::ExecuteLocalRepoRemoval { request_id, .. } => *request_id,
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
    LocalRepoRemovalPrepared {
        request_id: Uuid,
        preparation_id: Uuid,
        repo_id: RepoId,
        preview: LocalRepoRemovalPreview,
        confirmation_token: Option<RemovalConfirmationToken>,
        fallback_binding: Option<OpaqueFallbackBinding>,
        expires_at_unix_ms: Option<i64>,
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
    LocalRepoRemovalSettled {
        request_id: Uuid,
        job_id: Uuid,
        removed_repo_id: RepoId,
        final_repo_list: Vec<super::RepoListEntry>,
        scope: RepoRemovalFinalScope,
    },
    LocalRepoRemovalObserverInvalidated {
        job_id: Uuid,
        removed_repo_id: RepoId,
        final_repo_list: Vec<super::RepoListEntry>,
        scope: RepoRemovalFinalScope,
    },
    Error {
        request_id: Uuid,
        error: ServerError,
    },
}

fn is_hex_256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
#[path = "repo_control/tests.rs"]
mod tests;
