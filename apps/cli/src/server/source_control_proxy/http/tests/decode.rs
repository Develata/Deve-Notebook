//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use super::super::decode_error;
use deve_core::protocol::ServerErrorCode;
use reqwest::StatusCode;

#[test]
fn preserves_server_error_json() {
    let err = decode_error(
        StatusCode::BAD_REQUEST,
        br#"{"code":"SC_PENDING_NOT_FOUND","detail":"missing"}"#,
    );
    assert_eq!(err.code, ServerErrorCode::ScPendingNotFound);
    assert_eq!(err.detail.as_deref(), Some("missing"));
}

#[test]
fn wraps_plain_text_errors() {
    let err = decode_error(StatusCode::NOT_FOUND, b"notes/a.md");
    assert_eq!(err.code, ServerErrorCode::StorageNotFound);
    assert!(
        err.detail
            .as_deref()
            .is_some_and(|detail| detail.contains("404 Not Found"))
    );
}

#[test]
fn maps_db_locked_via_status_503() {
    let err = decode_error(StatusCode::SERVICE_UNAVAILABLE, b"server overloaded");
    assert_eq!(err.code, ServerErrorCode::StorageDbLocked);
}

#[test]
fn maps_unmapped_error_as_request_failed() {
    let err = decode_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        b"something completely unexpected",
    );
    assert_eq!(err.code, ServerErrorCode::RequestFailed);
}
