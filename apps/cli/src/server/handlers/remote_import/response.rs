//! plan_ref:
//!   - 07_network#remote-import-wire-contract
//!
//! Remote Import response construction and error classification.

use crate::remote_import_runtime::RemoteImportHostError;
use crate::server::channel::DualChannel;
use crate::server::repo_mutation::RepoMutationGateError;
use deve_core::protocol::{
    RemoteImportCandidateRevision, RemoteImportRequest, RemoteImportResponse,
    RemoteImportResponseContext, RemoteImportSessionId, ServerError, ServerErrorCode,
    ServerMessage,
};

pub(crate) async fn run_blocking<T>(
    operation: impl FnOnce() -> Result<T, RemoteImportHostError> + Send + 'static,
) -> Result<T, RemoteImportHostError>
where
    T: Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|error| {
            tracing::error!(%error, "Remote Import blocking task failed");
            RemoteImportHostError::Coordination
        })?
}

#[derive(Clone, Copy)]
pub(crate) enum RemoteImportPhase {
    Read,
    Prepare,
    Apply,
}

pub(super) fn send_host_error(
    ch: &DualChannel,
    context: RemoteImportResponseContext,
    error: RemoteImportHostError,
    phase: RemoteImportPhase,
) {
    tracing::warn!(%error, "Remote Import request failed");
    let code = host_error_code(&error, phase);
    send_error(ch, context, code);
}

pub(crate) fn host_error_code(
    error: &RemoteImportHostError,
    phase: RemoteImportPhase,
) -> ServerErrorCode {
    match error {
        RemoteImportHostError::Locator(_) => ServerErrorCode::RemoteProjectionLocatorInvalid,
        RemoteImportHostError::Provider(_) | RemoteImportHostError::ProviderCleanup { .. } => {
            ServerErrorCode::RemoteProjectionProviderUnavailable
        }
        RemoteImportHostError::ApplyBusy | RemoteImportHostError::Coordination => {
            phase_failure_code(phase)
        }
        RemoteImportHostError::Core(error) => map_core_error(error, phase),
    }
}

fn map_core_error(
    error: &deve_core::remote_import::RemoteImportError,
    phase: RemoteImportPhase,
) -> ServerErrorCode {
    use deve_core::remote_import::RemoteImportError as Error;
    match error {
        Error::ActiveSession(_) => ServerErrorCode::RemoteImportActiveSession,
        Error::SessionNotFound(_) => ServerErrorCode::RemoteImportNotFound,
        Error::Stale { .. }
        | Error::StaleGeneration(_)
        | Error::CandidateRevisionConflict { .. } => ServerErrorCode::RemoteImportStale,
        Error::Blocked { .. } => ServerErrorCode::RemoteImportBlocked,
        Error::InvalidState { .. } => ServerErrorCode::RemoteImportInvalidState,
        Error::LimitExceeded { .. } => ServerErrorCode::RemoteImportLimitExceeded,
        Error::RepairPlanChanged => ServerErrorCode::RemoteImportCleanupRequired,
        Error::ArtifactTampered(_) => ServerErrorCode::RemoteImportStale,
        _ => phase_failure_code(phase),
    }
}

fn phase_failure_code(phase: RemoteImportPhase) -> ServerErrorCode {
    match phase {
        RemoteImportPhase::Apply => ServerErrorCode::RemoteImportApplyFailed,
        RemoteImportPhase::Read | RemoteImportPhase::Prepare => {
            ServerErrorCode::RemoteImportPrepareFailed
        }
    }
}

pub(super) fn send_gate_error(
    ch: &DualChannel,
    context: RemoteImportResponseContext,
    error: RepoMutationGateError,
) {
    tracing::warn!(%error, "Remote Import mount gate rejected Apply");
    let error = if error == RepoMutationGateError::WorkspaceIngestionUnavailable {
        ServerError::workspace_ingestion_unavailable()
    } else {
        ServerError::new(ServerErrorCode::RemoteImportApplyFailed)
    };
    ch.unicast(ServerMessage::RemoteImport(RemoteImportResponse::Error {
        context,
        error,
    }));
}

pub(super) fn send_error(
    ch: &DualChannel,
    context: RemoteImportResponseContext,
    code: ServerErrorCode,
) {
    ch.unicast(ServerMessage::RemoteImport(RemoteImportResponse::Error {
        context,
        error: ServerError::new(code),
    }));
}

pub(super) fn response_context(
    context: &deve_core::protocol::RemoteImportRequestContext,
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
