//! plan_ref:
//!   - 08_auth#local-cli-proxy-authority
//!   - 14_commands#remote-import-command-contract
//!
//! Exact Remote Import request/response identities at the Local CLI boundary.

use deve_core::protocol::{
    RemoteImportCandidateRevision, RemoteImportRequest, RemoteImportRequestContext,
    RemoteImportResponseContext, RemoteImportSessionId,
};

pub(super) fn response_context(
    context: &RemoteImportRequestContext,
    identity: (
        Option<RemoteImportSessionId>,
        Option<RemoteImportCandidateRevision>,
    ),
) -> RemoteImportResponseContext {
    RemoteImportResponseContext {
        request_id: context.request_id,
        repo_id: context.repo_id,
        branch: context.branch.clone(),
        scope_nonce: context.scope_nonce,
        session_id: identity.0,
        revision: identity.1,
    }
}

pub(super) fn request_identity(
    request: &RemoteImportRequest,
) -> (
    Option<RemoteImportSessionId>,
    Option<RemoteImportCandidateRevision>,
) {
    match request {
        RemoteImportRequest::Prepare { .. } | RemoteImportRequest::List { .. } => (None, None),
        RemoteImportRequest::Show {
            session_id,
            revision,
            ..
        }
        | RemoteImportRequest::Discard {
            session_id,
            revision,
            ..
        } => (Some(*session_id), *revision),
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
        } => (Some(*session_id), Some(*revision)),
    }
}

pub(super) fn core_session_id(
    value: RemoteImportSessionId,
) -> deve_core::remote_import::RemoteImportSessionId {
    deve_core::remote_import::RemoteImportSessionId::from_uuid(value.get())
}

pub(super) fn core_revision(
    value: RemoteImportCandidateRevision,
) -> deve_core::remote_import::RemoteImportCandidateRevision {
    deve_core::remote_import::RemoteImportCandidateRevision::from_u64(value.get())
}
