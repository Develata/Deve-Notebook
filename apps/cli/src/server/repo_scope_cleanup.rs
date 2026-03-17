use deve_core::protocol::{ServerError, ServerErrorCode};

pub(super) fn should_clear_stale_remote_scope(error: &ServerError) -> bool {
    match error.code {
        ServerErrorCode::SyncRepoUnbound => true,
        ServerErrorCode::ScRepoContextInvalid => {
            let detail = error.detail.as_deref().unwrap_or_default();
            let lower = detail.to_ascii_lowercase();
            contains_any(
                &lower,
                &[
                    "remote session lost repo name",
                    "repository uuid not resolved",
                    "remote repository selector not resolved",
                    "session repo mismatch",
                    "repo selector mismatch",
                    "ambiguous remote repository selector",
                    "scope mismatch",
                    "stale scope nonce",
                ],
            )
        }
        _ => false,
    }
}

fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::should_clear_stale_remote_scope;
    use deve_core::protocol::{ServerError, ServerErrorCode};

    #[test]
    fn clears_unbound_remote_scope() {
        assert!(should_clear_stale_remote_scope(&ServerError::new(
            ServerErrorCode::SyncRepoUnbound
        )));
    }

    #[test]
    fn clears_stale_remote_selector_context_errors() {
        assert!(should_clear_stale_remote_scope(&ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            "Remote repository selector not resolved for wiki",
        )));
    }

    #[test]
    fn preserves_non_stale_remote_context_errors() {
        assert!(!should_clear_stale_remote_scope(&ServerError::with_detail(
            ServerErrorCode::ScRepoContextInvalid,
            "Local workspace path requested on remote branch: wiki",
        )));
    }
}
