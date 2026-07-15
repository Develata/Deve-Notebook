//! Runtime workspace-root discovery for baseline binaries.
//! plan_ref: infra

use anyhow::{Context, Result, bail};
use std::path::PathBuf;

pub(crate) fn repo_root() -> Result<PathBuf> {
    let current_dir = std::env::current_dir().context("failed to resolve current directory")?;
    repo_root_from_candidates([current_dir, PathBuf::from(env!("CARGO_MANIFEST_DIR"))])
}

fn repo_root_from_candidates(candidates: impl IntoIterator<Item = PathBuf>) -> Result<PathBuf> {
    for candidate in candidates {
        let mut cursor = candidate;
        loop {
            if cursor.join("Cargo.toml").is_file()
                && cursor.join("apps").is_dir()
                && cursor.join("crates").is_dir()
                && cursor.join("docs").is_dir()
            {
                return Ok(cursor);
            }
            if !cursor.pop() {
                break;
            }
        }
    }
    bail!("failed to resolve repository root from current directory or CARGO_MANIFEST_DIR")
}

#[cfg(test)]
mod tests {
    use super::{repo_root, repo_root_from_candidates};
    use std::path::PathBuf;

    #[test]
    fn resolves_workspace_root_from_runtime_or_baseline_manifest() {
        let root = repo_root().expect("repo root");

        assert!(root.join("Cargo.toml").is_file());
        assert!(root.join("apps/desktop/src/lib.rs").is_file());
        assert!(root.join("docs/acceptance-cases").is_dir());
    }

    #[test]
    fn falls_back_from_a_stale_compile_time_candidate() {
        let expected = repo_root().expect("repo root");
        let resolved = repo_root_from_candidates([
            PathBuf::from(r"Z:\deleted-worktree\tools\baseline"),
            expected.join("tools/baseline"),
        ])
        .expect("fallback repo root");

        assert_eq!(resolved, expected);
    }
}
