//! plan_ref:
//!   - 08_auth#local-cli-proxy-authority
//!   - 14_commands#remote-import-command-contract
//!
//! Process-local HTTP contract used only when the owner server holds a repo
//! database. It is not a browser or public Remote Import wire surface.

use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{
    RemoteImportRequest, RemoteImportRequestContext, RemoteImportResponse, ScopeNonce, ServerError,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
