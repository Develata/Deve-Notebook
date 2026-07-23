//! plan_ref:
//!   - 08_auth#local-cli-proxy-authority
//!   - 14_commands#remote-import-command-contract
//!
//! Process-local HTTP contract used only when the owner server holds a repo
//! database. It is not a browser or public Remote Import wire surface.

use crate::server::RepoRemovalRepairInspection;
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{
    LocalRepoRemovalPreview, OpaqueFallbackBinding, RemoteImportRequest,
    RemoteImportRequestContext, RemoteImportResponse, RemovalConfirmationToken,
    RepoLifecycleOperation, RepoLifecycleOutcome, RepoLifecycleState, ScopeNonce, ServerError,
    SwitchNonce,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(crate) const LOCAL_CLI_OWNER_HINT_FORMAT: &str = "deve.local-cli-owner";
pub(crate) const LOCAL_CLI_OWNER_HINT_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LocalCliOwnerHint {
    pub(crate) format: String,
    pub(crate) version: u8,
    pub(crate) main_port: u16,
    pub(crate) host_peer_id: String,
    pub(crate) runtime_incarnation: Uuid,
}

impl LocalCliOwnerHint {
    pub(crate) fn new(main_port: u16, host_peer_id: String, runtime_incarnation: Uuid) -> Self {
        Self {
            format: LOCAL_CLI_OWNER_HINT_FORMAT.into(),
            version: LOCAL_CLI_OWNER_HINT_VERSION,
            main_port,
            host_peer_id,
            runtime_incarnation,
        }
    }

    pub(crate) fn is_valid(&self) -> bool {
        self.format == LOCAL_CLI_OWNER_HINT_FORMAT
            && self.version == LOCAL_CLI_OWNER_HINT_VERSION
            && self.main_port != 0
            && !self.host_peer_id.trim().is_empty()
            && !self.runtime_incarnation.is_nil()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case")]
pub(crate) enum LocalCliRemoteImportRequest {
    Intent {
        request: RemoteImportRequest,
    },
    Repair {
        context: RemoteImportRequestContext,
        apply: bool,
    },
}

impl LocalCliRemoteImportRequest {
    pub(crate) fn request_id(&self) -> Uuid {
        match self {
            Self::Intent { request, .. } => request.context().request_id,
            Self::Repair { context, .. } => context.request_id,
        }
    }

    pub(crate) fn operation_name(&self) -> &'static str {
        match self {
            Self::Intent { request, .. } => match request {
                RemoteImportRequest::Prepare { .. } => "prepare",
                RemoteImportRequest::List { .. } => "list",
                RemoteImportRequest::Show { .. } => "show",
                RemoteImportRequest::Page { .. } => "page",
                RemoteImportRequest::Diff { .. } => "diff",
                RemoteImportRequest::Refresh { .. } => "refresh",
                RemoteImportRequest::Apply { .. } => "apply",
                RemoteImportRequest::Discard { .. } => "discard",
            },
            Self::Repair { apply: false, .. } => "repair-inspect",
            Self::Repair { apply: true, .. } => "repair-apply",
        }
    }

    pub(crate) fn exact_identity(&self) -> LocalCliRemoteImportIdentity<'_> {
        let context = match self {
            Self::Intent { request, .. } => request.context(),
            Self::Repair { context, .. } => context,
        };
        let (session_id, revision) = match self {
            Self::Intent { request, .. } => match request {
                RemoteImportRequest::Prepare { .. } | RemoteImportRequest::List { .. } => {
                    (None, None)
                }
                RemoteImportRequest::Show {
                    session_id,
                    revision,
                    ..
                }
                | RemoteImportRequest::Discard {
                    session_id,
                    revision,
                    ..
                } => (Some(session_id.get()), revision.map(|value| value.get())),
                RemoteImportRequest::Page {
                    session_id,
                    revision,
                    ..
                }
                | RemoteImportRequest::Diff {
                    session_id,
                    revision,
                    ..
                }
                | RemoteImportRequest::Refresh {
                    session_id,
                    revision,
                    ..
                }
                | RemoteImportRequest::Apply {
                    session_id,
                    revision,
                    ..
                } => (Some(session_id.get()), Some(revision.get())),
            },
            Self::Repair { .. } => (None, None),
        };
        LocalCliRemoteImportIdentity {
            repo_id: context.repo_id,
            branch: context.branch.as_ref(),
            scope_nonce: context.scope_nonce,
            session_id,
            revision,
        }
    }
}

pub(crate) struct LocalCliRemoteImportIdentity<'a> {
    pub(crate) repo_id: RepoId,
    pub(crate) branch: Option<&'a PeerId>,
    pub(crate) scope_nonce: ScopeNonce,
    pub(crate) session_id: Option<Uuid>,
    pub(crate) revision: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "kebab-case")]
pub(crate) enum LocalCliRemoteImportResponse {
    Intent {
        response: RemoteImportResponse,
    },
    Repair {
        request_id: Uuid,
        repo_id: RepoId,
        branch: Option<PeerId>,
        scope_nonce: ScopeNonce,
        finding_count: usize,
        repairable_count: usize,
    },
    Error {
        request_id: Uuid,
        error: ServerError,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case")]
pub(crate) enum LocalCliRepoRemovalRequest {
    Prepare {
        request_id: Uuid,
        repo_id: RepoId,
        current_scope_nonce: ScopeNonce,
    },
    Execute {
        request_id: Uuid,
        repo_id: RepoId,
        preparation_id: Uuid,
        confirmation_token: RemovalConfirmationToken,
        fallback_binding: Option<OpaqueFallbackBinding>,
        current_scope_nonce: ScopeNonce,
        switch_nonce: SwitchNonce,
    },
    Status {
        request_id: Uuid,
        execute_request_id: Uuid,
        repo_id: RepoId,
    },
    RepairPrepare {
        request_id: Uuid,
    },
    RepairApply {
        request_id: Uuid,
        token: String,
    },
}

impl LocalCliRepoRemovalRequest {
    pub(crate) const fn request_id(&self) -> Uuid {
        match self {
            Self::Prepare { request_id, .. }
            | Self::Execute { request_id, .. }
            | Self::Status { request_id, .. }
            | Self::RepairPrepare { request_id }
            | Self::RepairApply { request_id, .. } => *request_id,
        }
    }

    pub(crate) const fn repo_id(&self) -> RepoId {
        match self {
            Self::Prepare { repo_id, .. }
            | Self::Execute { repo_id, .. }
            | Self::Status { repo_id, .. } => *repo_id,
            Self::RepairPrepare { .. } | Self::RepairApply { .. } => RepoId::nil(),
        }
    }

    pub(crate) const fn operation_name(&self) -> &'static str {
        match self {
            Self::Prepare { .. } => "prepare",
            Self::Execute { .. } => "execute",
            Self::Status { .. } => "status",
            Self::RepairPrepare { .. } => "repair-prepare",
            Self::RepairApply { .. } => "repair-apply",
        }
    }

    pub(crate) const fn scope_identity(&self) -> (u64, Option<u64>, Option<Uuid>) {
        match self {
            Self::Prepare {
                current_scope_nonce,
                ..
            } => (current_scope_nonce.get(), None, None),
            Self::Execute {
                current_scope_nonce,
                switch_nonce,
                preparation_id,
                ..
            } => (
                current_scope_nonce.get(),
                Some(switch_nonce.get()),
                Some(*preparation_id),
            ),
            Self::Status {
                execute_request_id, ..
            } => (0, None, Some(*execute_request_id)),
            Self::RepairPrepare { request_id } | Self::RepairApply { request_id, .. } => {
                (0, None, Some(*request_id))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "kebab-case")]
pub(crate) enum LocalCliRepoRemovalResponse {
    Prepared {
        request_id: Uuid,
        preparation_id: Uuid,
        repo_id: RepoId,
        preview: LocalRepoRemovalPreview,
        confirmation_token: Option<RemovalConfirmationToken>,
        fallback_binding: Option<OpaqueFallbackBinding>,
    },
    Accepted {
        request_id: Uuid,
        job_id: Uuid,
        repo_id: RepoId,
    },
    Status {
        request_id: Uuid,
        execute_request_id: Uuid,
        job_id: Uuid,
        repo_id: RepoId,
        operation: RepoLifecycleOperation,
        state: RepoLifecycleState,
        outcome: Option<RepoLifecycleOutcome>,
        publication_pending: bool,
    },
    RepairPrepared {
        request_id: Uuid,
        inspection: RepoRemovalRepairInspection,
        token: Option<String>,
        expires_at_unix_ms: Option<i64>,
    },
    Error {
        request_id: Uuid,
        error: ServerError,
    },
}
