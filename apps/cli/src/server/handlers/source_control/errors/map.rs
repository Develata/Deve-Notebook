//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Source-control error mapping entry points.

#[path = "map_common.rs"]
mod common;
#[path = "map_op.rs"]
mod op;
#[path = "map_op_specific.rs"]
mod op_specific;

use anyhow::Error;
use deve_core::protocol::{ServerError, ServerErrorCode};
pub use op::ScOp;

pub fn map_repo_scope_error(error: Error) -> ServerError {
    if let Some(error) = crate::server::repo_scope::RepoScopeFailure::from_anyhow(&error) {
        return source_control_scope_failure(error);
    }
    let detail = error.to_string();
    if let Some(code) = common::classify_common_scope_code(&detail) {
        return ServerError::with_detail(code, detail);
    }
    ServerError::with_detail(ServerErrorCode::RequestFailed, detail)
}

pub fn map_repo_error(op: ScOp, error: Error) -> ServerError {
    if let Some(error) = crate::server::repo_scope::RepoScopeFailure::from_anyhow(&error) {
        return source_control_scope_failure(error);
    }
    let detail = error.to_string();
    if let Ok(error) = serde_json::from_str::<ServerError>(&detail) {
        return error;
    }
    if let Some(error) = op_specific::classify_op_specific_error(&op, &detail) {
        return error;
    }
    if let Some(code) = common::classify_common_scope_code(&detail) {
        return ServerError::with_detail(code, detail);
    }
    if detail.to_ascii_lowercase().contains("conflict") {
        return ServerError::with_detail(ServerErrorCode::StorageConflict, detail);
    }
    ServerError::with_detail(ServerErrorCode::RequestFailed, detail)
}

fn source_control_scope_failure(
    error: &crate::server::repo_scope::RepoScopeFailure,
) -> ServerError {
    let code = match error {
        crate::server::repo_scope::RepoScopeFailure::RepoUnbound { .. } => {
            ServerErrorCode::ScRepoNotSelected
        }
        _ => error.code(),
    };
    ServerError::with_detail(code, error.detail())
}

#[cfg(test)]
#[path = "map_op_test.rs"]
mod op_tests;

#[cfg(test)]
#[path = "map_scope_test.rs"]
mod scope_tests;
