mod map;

use crate::server::channel::DualChannel;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use deve_core::protocol::{ServerError, ServerErrorCode};

pub use map::{ScOp, map_repo_error, map_repo_scope_error};

pub fn send_ws(ch: &DualChannel, error: ServerError) {
    ch.send_protocol_error(error);
}

pub fn send_ws_code(ch: &DualChannel, code: ServerErrorCode, detail: impl Into<String>) {
    send_ws(ch, ServerError::with_detail(code, detail));
}

pub fn http(error: ServerError) -> Response {
    (status(error.code), Json(error)).into_response()
}

pub fn request_failed(detail: impl Into<String>) -> ServerError {
    ServerError::with_detail(ServerErrorCode::RequestFailed, detail)
}

fn status(code: ServerErrorCode) -> StatusCode {
    match code {
        ServerErrorCode::ScDocNotFound | ServerErrorCode::ScCommitNotFound => StatusCode::NOT_FOUND,
        ServerErrorCode::StorageDbLocked => StatusCode::SERVICE_UNAVAILABLE,
        ServerErrorCode::AuthTokenExpired | ServerErrorCode::AuthTokenMissing => {
            StatusCode::UNAUTHORIZED
        }
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
