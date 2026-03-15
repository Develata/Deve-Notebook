pub(super) fn should_clear_stale_remote_scope(detail: &str) -> bool {
    let lower = detail.to_ascii_lowercase();
    lower.contains("active repository not selected")
        || contains_any(
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

fn contains_any(input: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| input.contains(pattern))
}
