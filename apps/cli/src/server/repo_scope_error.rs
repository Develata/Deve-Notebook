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
            "remote session lost repo name",
            "repository uuid not resolved",
            "session repo mismatch",
            "repo selector mismatch",
            "local repo not found for uuid",
        ],
    ) {
        return ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, detail);
    }
    ServerError::with_detail(ServerErrorCode::RequestFailed, detail)
}

fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
}
