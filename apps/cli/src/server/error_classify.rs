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
                "failed to stat local repo directory",
                "failed to read repo entry type",
                "failed to stat remote peer directory",
                "failed to read remote peer directory metadata",
                "failed to stat remote repo directory",
                "failed to stat remote repo path candidate",
                "deserialize",
                "decode",
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
            "local repo not found for uuid",
            "local repo operation requested on remote branch",
            "local workspace path requested on remote branch",
            "local workspace root requested on remote branch",
            "scope mismatch",
            "stale scope nonce",
        ],
    )
}

fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::is_storage_corruption;

    #[test]
    fn classifies_remote_peer_directory_stat_errors_as_storage_corruption() {
        assert!(is_storage_corruption(
            "failed to stat remote peer directory \"/tmp/remotes/peer-a\" while validating branch availability: permission denied",
        ));
    }

    #[test]
    fn classifies_remote_peer_directory_metadata_errors_as_storage_corruption() {
        assert!(is_storage_corruption(
            "failed to read remote peer directory metadata \"/tmp/remotes/peer-a\" while validating branch availability: input/output error",
        ));
    }
}
