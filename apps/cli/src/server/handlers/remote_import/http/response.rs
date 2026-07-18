//! plan_ref:
//!   - 08_auth#local-cli-proxy-authority
//!   - 14_commands#remote-import-command-contract
//!
//! Local CLI proxy response envelopes and stable HTTP status mapping.

use super::super::{RemoteImportPhase, host_error_code};
use crate::local_cli_proxy_contract::LocalCliRemoteImportResponse;
use crate::remote_import_runtime::RemoteImportHostError;
use crate::server::repo_mutation::RepoMutationGateError;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use deve_core::protocol::auth::{AuthErrorCode, AuthErrorResponse};
use deve_core::protocol::{
    RemoteImportResponse, RemoteImportResponseContext, ServerError, ServerErrorCode,
};

pub(super) fn intent_response(response: RemoteImportResponse) -> Response {
    let status = match &response {
        RemoteImportResponse::Error { error, .. } => status_for(error.code),
        _ => StatusCode::OK,
    };
    (
        status,
        Json(LocalCliRemoteImportResponse::Intent { response }),
    )
        .into_response()
}

pub(super) fn intent_error(
    context: RemoteImportResponseContext,
    code: ServerErrorCode,
) -> Response {
    intent_response(RemoteImportResponse::Error {
        context,
        error: ServerError::new(code),
    })
}

pub(super) fn host_error(
    context: RemoteImportResponseContext,
    error: RemoteImportHostError,
    phase: RemoteImportPhase,
) -> RemoteImportResponse {
    tracing::warn!(%error, "Local CLI Remote Import operation failed");
    RemoteImportResponse::Error {
        context,
        error: ServerError::new(host_error_code(&error, phase)),
    }
}

pub(super) fn gate_error(
    context: RemoteImportResponseContext,
    error: RepoMutationGateError,
) -> RemoteImportResponse {
    tracing::warn!(%error, "Local CLI Remote Import mount gate rejected Apply");
    RemoteImportResponse::Error {
        context,
        error: if error == RepoMutationGateError::WorkspaceIngestionUnavailable {
            ServerError::workspace_ingestion_unavailable()
        } else {
            ServerError::new(ServerErrorCode::RemoteImportApplyFailed)
        },
    }
}

pub(super) fn proxy_error(request_id: uuid::Uuid, code: ServerErrorCode) -> Response {
    (
        status_for(code),
        Json(LocalCliRemoteImportResponse::Error {
            request_id,
            error: ServerError::new(code),
        }),
    )
        .into_response()
}

pub(super) fn auth_internal_error() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(AuthErrorResponse::new(AuthErrorCode::InternalError)),
    )
        .into_response()
}

fn status_for(code: ServerErrorCode) -> StatusCode {
    match code {
        ServerErrorCode::StorageWorkspaceIngestionUnavailable
        | ServerErrorCode::RemoteProjectionProviderUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        ServerErrorCode::RemoteImportNotFound => StatusCode::NOT_FOUND,
        ServerErrorCode::RemoteProjectionLocatorInvalid => StatusCode::CONFLICT,
        ServerErrorCode::RemoteProjectionPushFailed => StatusCode::BAD_GATEWAY,
        ServerErrorCode::RemoteImportLimitExceeded => StatusCode::PAYLOAD_TOO_LARGE,
        ServerErrorCode::RemoteImportActiveSession
        | ServerErrorCode::RemoteImportStale
        | ServerErrorCode::RemoteImportBlocked
        | ServerErrorCode::RemoteImportInvalidState
        | ServerErrorCode::RemoteImportCleanupRequired => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_import_status_codes_preserve_transport_contract() {
        assert_eq!(
            status_for(ServerErrorCode::RemoteImportInvalidState),
            StatusCode::CONFLICT
        );
        assert_eq!(
            status_for(ServerErrorCode::StorageWorkspaceIngestionUnavailable),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            status_for(ServerErrorCode::RemoteProjectionLocatorInvalid),
            StatusCode::CONFLICT
        );
        assert_eq!(
            status_for(ServerErrorCode::RemoteProjectionProviderUnavailable),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            status_for(ServerErrorCode::RemoteProjectionPushFailed),
            StatusCode::BAD_GATEWAY
        );
        assert_eq!(
            status_for(ServerErrorCode::RemoteImportLimitExceeded),
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }
}
