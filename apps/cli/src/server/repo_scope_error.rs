use anyhow::Error;
use deve_core::protocol::{ServerError, ServerErrorCode};

pub fn map_repo_scope_error(error: Error) -> ServerError {
    let detail = error.to_string();
    let lower = detail.to_ascii_lowercase();
    if lower.contains("active repository not selected") {
        return ServerError::with_detail(ServerErrorCode::SyncRepoUnbound, detail);
    }
    if contains_any(
        &lower,
        &[
            "repository not found:",
            "document not found",
            "doc not found",
        ],
    ) {
        return ServerError::with_detail(ServerErrorCode::StorageNotFound, detail);
    }
    if contains_any(
        &lower,
        &[
            "database already open",
            "cannot acquire lock",
            "db locked",
            "database is locked",
        ],
    ) {
        return ServerError::with_detail(ServerErrorCode::StorageDbLocked, detail);
    }
    if contains_any(
        &lower,
        &[
            "remote session lost repo name",
            "repository uuid not resolved",
            "remote repository selector not resolved",
            "local repository uuid not resolved",
            "session repo mismatch",
            "repo selector mismatch",
            "ambiguous local repository selector",
            "local repo not found for uuid",
            "scope mismatch",
            "stale scope nonce",
        ],
    ) {
        return ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, detail);
    }
    ServerError::with_detail(ServerErrorCode::RequestFailed, detail)
}

fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
}
