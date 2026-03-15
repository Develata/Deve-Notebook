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
            "cannot bootstrap local repo while on remote branch",
            "repository uuid not resolved",
            "remote repository selector not resolved",
            "local repository uuid not resolved",
            "session repo mismatch",
            "repo selector mismatch",
            "ambiguous local repository selector",
            "ambiguous remote repository selector",
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

#[cfg(test)]
mod tests {
    use super::map_repo_scope_error;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn classifies_ambiguous_remote_selector_as_repo_context_invalid() {
        let err = map_repo_scope_error(anyhow::anyhow!(
            "Ambiguous remote repository selector: shadow-wiki"
        ));
        assert_eq!(err.code, ServerErrorCode::ScRepoContextInvalid);
    }
}
