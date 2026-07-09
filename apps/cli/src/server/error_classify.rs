//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Shared server error-string classification helpers.

pub(super) fn is_repo_not_selected(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "active repository not selected",
            "multiple local repos exist",
            "no local repositories available",
        ],
    )
}

pub(super) fn is_storage_not_found(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "repository not found:",
            "document not found",
            "doc not found",
            "local repo not found for name",
        ],
    )
}

pub(super) fn is_db_locked(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "database already open",
            "cannot acquire lock",
            "db locked",
            "database is locked",
            "failed to lock database",
        ],
    )
}

pub(super) fn is_storage_corruption(lower: &str) -> bool {
    lower.contains("tracked document projection missing")
        || contains_any(
            lower,
            &[
                "broken remote repo",
                "broken remote repo catalog",
                "remote repo directory missing",
                "broken repo entry",
                "broken local repo",
                "broken shadow repo",
                "broken shadow peer",
                "failed to walk local repo",
                "failed to list local repos",
                "failed to stat local repo directory",
                "failed to read repo entry type",
                "failed to stat remote peer directory",
                "failed to read remote peer directory metadata",
                "failed to stat remote repo directory",
                "failed to stat remote repo path candidate",
                "tree registry lock poisoned",
                "repotreeregistry write lock poisoned",
                "local repo registry lock poisoned",
                "shadow db registry lock poisoned",
                "database cache lock poisoned",
                "reposcopedsyncengine registry poisoned",
                "reposcopedsyncengine read lock poisoned",
                "reposcopedsyncengine write lock poisoned",
                "deserialize",
                "deserialization",
                "decode",
                "postcard deserialization failed",
                "unexpected end",
            ],
        )
}

pub(super) fn is_repo_context_invalid(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "stale remote scope:",
            "remote branch not available:",
            "remote session lost repo name",
            "cannot bootstrap local repo while on remote branch",
            "repository uuid not resolved",
            "remote repository selector not resolved",
            "local repository selector not resolved",
            "local repository uuid not resolved",
            "session repo mismatch",
            "repo selector mismatch",
            "ambiguous local repository selector",
            "ambiguous remote repository selector",
            "metadata name drifted",
            "local repo not found for uuid",
            "local repo operation requested on remote branch",
            "local workspace path requested on remote branch",
            "local workspace root requested on remote branch",
            "scope mismatch",
        ],
    )
}

pub(super) fn is_remote_exact_selector_mismatch(lower: &str) -> bool {
    lower.contains("stale remote scope:")
        && lower.contains("session repo mismatch")
        && lower.contains("exact repository selector")
}

pub(super) fn is_stale_scope(lower: &str) -> bool {
    contains_any(lower, &["stale scope nonce", "stale remote scope:"])
}

pub(super) fn is_repo_route_mismatch(lower: &str) -> bool {
    contains_any(lower, &["sync repo mismatch", "repo route mismatch"])
}

pub(super) fn is_invalid_sync_payload(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "invalid sync payload",
            "encrypted op seq mismatch",
            "sync payload peer/repo mismatch",
        ],
    )
}

fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
}

#[cfg(test)]
mod tests;
