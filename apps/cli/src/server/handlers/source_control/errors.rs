//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 06_repository#repo-scope-runtime

mod map;

use crate::server::channel::DualChannel;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use deve_core::protocol::{ServerError, ServerErrorCode};

pub use map::{ScOp, map_repo_error, map_repo_scope_error};

pub fn send_ws_scoped(ch: &DualChannel, error: ServerError, scope_nonce: Option<u64>) {
    ch.send_protocol_error_with_scope_nonce(error, scope_nonce);
}

pub fn send_ws_code_scoped(
    ch: &DualChannel,
    code: ServerErrorCode,
    detail: impl Into<String>,
    scope_nonce: Option<u64>,
) {
    send_ws_scoped(ch, ServerError::with_detail(code, detail), scope_nonce);
}

pub fn http(error: ServerError) -> Response {
    (status(error.code), Json(error)).into_response()
}

pub fn unsupported(detail: impl Into<String>) -> ServerError {
    ServerError::with_detail(ServerErrorCode::PluginUnsupportedMessage, detail)
}

fn status(code: ServerErrorCode) -> StatusCode {
    match code {
        ServerErrorCode::ScDocNotFound
        | ServerErrorCode::ScCommitNotFound
        | ServerErrorCode::DocNotFound
        | ServerErrorCode::PluginUnknownPlugin => StatusCode::NOT_FOUND,
        ServerErrorCode::StorageDbLocked | ServerErrorCode::SyncDisconnected => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        ServerErrorCode::AuthTokenExpired | ServerErrorCode::AuthTokenMissing => {
            StatusCode::UNAUTHORIZED
        }
        ServerErrorCode::AuthInvalidPassword => StatusCode::UNAUTHORIZED,
        ServerErrorCode::AuthRateLimited => StatusCode::TOO_MANY_REQUESTS,
        ServerErrorCode::AuthCsrfMismatch | ServerErrorCode::PluginCapabilityDenied => {
            StatusCode::FORBIDDEN
        }
        ServerErrorCode::SyncInvalidPayload
        | ServerErrorCode::PluginInvalidMessage
        | ServerErrorCode::PluginUnsupportedMessage => StatusCode::BAD_REQUEST,
        ServerErrorCode::ScRepoNotSelected
        | ServerErrorCode::ScRemoteBranchReadonly
        | ServerErrorCode::ScRepoContextInvalid
        | ServerErrorCode::ScStaleScope
        | ServerErrorCode::ScPendingNotFound
        | ServerErrorCode::ScStagedNotFound
        | ServerErrorCode::ScCommitDiffUnprojectable
        | ServerErrorCode::ScNothingToCommit
        | ServerErrorCode::ScConflictTargetMissing
        | ServerErrorCode::DocContextInvalid
        | ServerErrorCode::SyncRepoUnbound
        | ServerErrorCode::SyncRepoRouteMismatch
        | ServerErrorCode::SyncSnapshotRequired
        | ServerErrorCode::SyncVersionMismatch
        | ServerErrorCode::StorageConflict
        | ServerErrorCode::SyncEditRejected => StatusCode::CONFLICT,
        ServerErrorCode::SyncPeerUnauthenticated | ServerErrorCode::SyncPeerUnknown => {
            StatusCode::FORBIDDEN
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[cfg(test)]
mod tests {
    use super::{status, unsupported};
    use axum::http::StatusCode;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn plugin_unsupported_errors_map_to_bad_request() {
        let err = unsupported("Repository not configured");
        assert_eq!(err.code, ServerErrorCode::PluginUnsupportedMessage);
        assert_eq!(status(err.code), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn plan_catalog_statuses_cover_new_structured_codes() {
        assert_eq!(status(ServerErrorCode::ScStaleScope), StatusCode::CONFLICT);
        assert_eq!(
            status(ServerErrorCode::DocContextInvalid),
            StatusCode::CONFLICT
        );
        assert_eq!(
            status(ServerErrorCode::SyncRepoRouteMismatch),
            StatusCode::CONFLICT
        );
        assert_eq!(
            status(ServerErrorCode::SyncInvalidPayload),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            status(ServerErrorCode::PluginUnknownPlugin),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            status(ServerErrorCode::PluginCapabilityDenied),
            StatusCode::FORBIDDEN
        );
    }
}
