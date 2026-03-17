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
        ServerErrorCode::ScDocNotFound | ServerErrorCode::ScCommitNotFound => StatusCode::NOT_FOUND,
        ServerErrorCode::StorageDbLocked => StatusCode::SERVICE_UNAVAILABLE,
        ServerErrorCode::AuthTokenExpired | ServerErrorCode::AuthTokenMissing => {
            StatusCode::UNAUTHORIZED
        }
        ServerErrorCode::PluginUnsupportedMessage => StatusCode::NOT_IMPLEMENTED,
        ServerErrorCode::ScRepoNotSelected
        | ServerErrorCode::ScRemoteBranchReadonly
        | ServerErrorCode::ScRepoContextInvalid
        | ServerErrorCode::ScPendingNotFound
        | ServerErrorCode::ScStagedNotFound
        | ServerErrorCode::ScNothingToCommit
        | ServerErrorCode::ScConflictTargetMissing
        | ServerErrorCode::SyncRepoUnbound
        | ServerErrorCode::StorageConflict
        | ServerErrorCode::SyncEditRejected => StatusCode::CONFLICT,
        ServerErrorCode::SyncPeerUnauthenticated => StatusCode::FORBIDDEN,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[cfg(test)]
mod tests {
    use super::{status, unsupported};
    use axum::http::StatusCode;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn plugin_unsupported_errors_map_to_not_implemented() {
        let err = unsupported("Repository not configured");
        assert_eq!(err.code, ServerErrorCode::PluginUnsupportedMessage);
        assert_eq!(status(err.code), StatusCode::NOT_IMPLEMENTED);
    }
}
