//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!
//! Source-control HTTP scope nonce gate.

use deve_core::protocol::{ServerError, ServerErrorCode};

pub(super) fn require(scope_nonce: Option<u64>) -> Result<u64, ServerError> {
    match scope_nonce {
        Some(scope_nonce) if scope_nonce > 0 => Ok(scope_nonce),
        _ => Err(ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            "source control scope nonce missing",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::require;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn source_control_http_scope_requires_nonzero_nonce() {
        assert!(require(Some(1)).is_ok());
        for value in [None, Some(0)] {
            let err = require(value).expect_err("missing scope must fail closed");
            assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
            assert_eq!(
                err.detail.as_deref(),
                Some("source control scope nonce missing")
            );
        }
    }
}
