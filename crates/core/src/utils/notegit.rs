//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout

use crate::models::RepoId;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

mod removal;
pub use removal::{NotegitRemovalCheckpoint, NotegitRemovalPlan, prepare_removal};

pub const NOTE_GIT_DIR: &str = ".notegit";
pub const GIT_DIR: &str = ".git";
pub const NOTE_GIT_IGNORE_PATTERN: &str = ".notegit/";
const DEVE_IGNORE_FILE: &str = ".deveignore";
const IDENTITY_FILE: &str = "identity.toml";
const IDENTITY_VERSION: u32 = 1;
const IDENTITY_MARKER_MAX_BYTES: u64 = 64 * 1024;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RepoIdentityMarker {
    version: u32,
    repo_id: RepoId,
    repo_name: String,
}

pub fn repo_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(NOTE_GIT_DIR)
}

pub fn repo_identity_path(repo_root: &Path) -> PathBuf {
    repo_dir(repo_root).join(IDENTITY_FILE)
}

pub fn is_internal_repo_path(path: &str) -> bool {
    crate::utils::path::to_forward_slash(path)
        .split('/')
        .any(is_internal_repo_segment)
}

pub fn is_internal_repo_segment(segment: &str) -> bool {
    segment.eq_ignore_ascii_case(NOTE_GIT_DIR) || segment.eq_ignore_ascii_case(GIT_DIR)
}

pub fn repo_keys_dir(repo_root: &Path) -> PathBuf {
    repo_dir(repo_root).join("keys")
}

pub fn ensure_repo_identity_marker(
    repo_root: &Path,
    repo_id: RepoId,
    repo_name: &str,
) -> Result<()> {
    let path = repo_identity_path(repo_root);
    if let Some(marker) = read_repo_identity_marker(&path)? {
        validate_repo_identity(repo_root, &marker, repo_id)?;
        if marker.repo_name == repo_name {
            return Ok(());
        }
    } else if workspace_has_external_content(repo_root)? {
        return Err(anyhow!(
            "Projection workspace {:?} is missing .notegit identity marker for repo {} and already contains non-internal content",
            repo_root,
            repo_id
        ));
    }
    write_repo_identity_marker(repo_root, repo_id, repo_name)
}

pub fn validate_repo_identity_marker(repo_root: &Path, repo_id: RepoId) -> Result<()> {
    let path = repo_identity_path(repo_root);
    let marker = read_repo_identity_marker(&path)?.ok_or_else(|| {
        anyhow!(
            "Projection workspace {:?} is missing .notegit identity marker",
            repo_root
        )
    })?;
    validate_repo_identity(repo_root, &marker, repo_id)
}

/// Validates an identity marker already read through a caller-owned no-follow handle.
///
/// This keeps parsing and RepoId semantics project-owned while allowing destructive
/// admission paths to bind the validated bytes to an exact open-file identity.
pub fn validate_repo_identity_marker_content(
    content: &[u8],
    repo_root: &Path,
    repo_id: RepoId,
) -> Result<()> {
    let content = std::str::from_utf8(content).context("repo identity marker is not UTF-8")?;
    let marker: RepoIdentityMarker = toml::from_str(content)
        .with_context(|| format!("Failed to parse repo identity marker: {:?}", repo_root))?;
    validate_repo_identity_marker_version(&marker, repo_root)?;
    validate_repo_identity(repo_root, &marker, repo_id)
}

fn write_repo_identity_marker(repo_root: &Path, repo_id: RepoId, repo_name: &str) -> Result<()> {
    let dir = repo_dir(repo_root);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create .notegit directory: {:?}", dir))?;
    let marker = RepoIdentityMarker {
        version: IDENTITY_VERSION,
        repo_id,
        repo_name: repo_name.to_string(),
    };
    let content =
        toml::to_string_pretty(&marker).context("Failed to serialize repo identity marker")?;
    let path = repo_identity_path(repo_root);
    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write repo identity marker: {:?}", path))
}

fn read_repo_identity_marker(path: &Path) -> Result<Option<RepoIdentityMarker>> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(err)
                .with_context(|| format!("Failed to stat repo identity marker: {:?}", path));
        }
    };
    if meta.file_type().is_symlink() {
        return Err(anyhow!(
            "refusing to follow symlinked repo identity marker {:?}",
            path
        ));
    }
    if !meta.is_file() {
        return Err(anyhow!(
            "repo identity marker is not a regular file {:?}",
            path
        ));
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read repo identity marker: {:?}", path))?;
    let marker: RepoIdentityMarker = toml::from_str(&content)
        .with_context(|| format!("Failed to parse repo identity marker: {:?}", path))?;
    validate_repo_identity_marker_version(&marker, path)?;
    Ok(Some(marker))
}

fn validate_repo_identity_marker_version(marker: &RepoIdentityMarker, path: &Path) -> Result<()> {
    if marker.version != IDENTITY_VERSION {
        return Err(anyhow!(
            "Unsupported repo identity marker version {} in {:?}",
            marker.version,
            path
        ));
    }
    Ok(())
}

fn validate_repo_identity(
    repo_root: &Path,
    marker: &RepoIdentityMarker,
    expected_repo_id: RepoId,
) -> Result<()> {
    if marker.repo_id != expected_repo_id {
        return Err(anyhow!(
            "Projection workspace {:?} identity marker repo_id mismatch: expected {}, got {}",
            repo_root,
            expected_repo_id,
            marker.repo_id
        ));
    }
    Ok(())
}

fn workspace_has_external_content(repo_root: &Path) -> Result<bool> {
    match repo_root.try_exists() {
        Ok(false) => return Ok(false),
        Ok(true) => {}
        Err(err) => {
            return Err(err).with_context(|| {
                format!(
                    "Failed to stat Projection workspace before writing identity marker: {:?}",
                    repo_root
                )
            });
        }
    }
    for entry in std::fs::read_dir(repo_root)
        .with_context(|| format!("Failed to read Projection workspace: {:?}", repo_root))?
    {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if is_internal_repo_segment(name.as_ref())
            || name == ".gitignore"
            || name == DEVE_IGNORE_FILE
        {
            continue;
        }
        return Ok(true);
    }
    Ok(false)
}

pub fn host_dir(ledger_root: &Path) -> PathBuf {
    ledger_root.join(".host")
}

pub fn host_keys_dir(ledger_root: &Path) -> PathBuf {
    host_dir(ledger_root).join("keys")
}

pub fn gitignore_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".gitignore")
}

pub fn gitignore_ignores_notegit(repo_root: &Path) -> std::io::Result<bool> {
    let path = gitignore_path(repo_root);
    let Some(content) = read_regular_gitignore(&path)? else {
        return Ok(false);
    };
    Ok(gitignore_effectively_ignores_notegit(&content))
}

pub fn ensure_gitignore_ignores_notegit(repo_root: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(repo_root)?;
    let path = gitignore_path(repo_root);
    let existing = read_regular_gitignore(&path)?.unwrap_or_default();
    if gitignore_effectively_ignores_notegit(&existing) {
        return Ok(());
    }

    let mut next = existing;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next.push_str(NOTE_GIT_IGNORE_PATTERN);
    next.push('\n');
    std::fs::write(path, next)
}

fn read_regular_gitignore(path: &Path) -> std::io::Result<Option<String>> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };
    if meta.file_type().is_symlink() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("refusing to follow symlinked .gitignore {}", path.display()),
        ));
    }
    if !meta.is_file() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(".gitignore is not a regular file {}", path.display()),
        ));
    }
    std::fs::read_to_string(path).map(Some)
}

fn gitignore_effectively_ignores_notegit(content: &str) -> bool {
    let mut protected = false;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (negated, pattern) = match line.strip_prefix('!') {
            Some(pattern) => (true, pattern.trim()),
            None => (false, line),
        };
        if gitignore_pattern_mentions_notegit(pattern) {
            protected = !negated;
        }
    }
    protected
}

fn gitignore_pattern_mentions_notegit(pattern: &str) -> bool {
    let pattern = pattern.trim_start_matches('/');
    pattern == ".notegit" || pattern.starts_with(".notegit/")
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_gitignore_ignores_notegit, ensure_repo_identity_marker, gitignore_ignores_notegit,
        is_internal_repo_path, validate_repo_identity_marker,
    };

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    #[test]
    fn internal_repo_path_uses_segment_semantics() {
        assert!(is_internal_repo_path(".notegit/state.json"));
        assert!(is_internal_repo_path("notes/.notegit/state.json"));
        assert!(is_internal_repo_path(".git/config"));
        assert!(is_internal_repo_path("notes/.git/objects/x"));
        assert!(is_internal_repo_path("notes/.NOTEGIT/state.json"));
        assert!(is_internal_repo_path("notes/.GIT/config"));
        assert!(!is_internal_repo_path(".gitignore"));
        assert!(!is_internal_repo_path(".notegit-backup/state.json"));
        assert!(!is_internal_repo_path(".git-backup/config"));
    }

    #[test]
    fn ensure_gitignore_ignores_notegit_is_idempotent() {
        let dir = tempfile::tempdir().expect("tempdir");

        ensure_gitignore_ignores_notegit(dir.path()).expect("ensure");
        ensure_gitignore_ignores_notegit(dir.path()).expect("ensure again");

        let content = std::fs::read_to_string(dir.path().join(".gitignore")).expect("read");
        assert_eq!(content.matches(".notegit/").count(), 1);
        assert!(gitignore_ignores_notegit(dir.path()).expect("status"));
    }

    #[test]
    fn existing_root_notegit_gitignore_pattern_counts_as_protected() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".gitignore"), "/.notegit/\n").expect("write");

        ensure_gitignore_ignores_notegit(dir.path()).expect("ensure");

        let content = std::fs::read_to_string(dir.path().join(".gitignore")).expect("read");
        assert_eq!(content, "/.notegit/\n");
        assert!(gitignore_ignores_notegit(dir.path()).expect("status"));
    }

    #[test]
    fn later_gitignore_negation_is_not_reported_as_protected() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".gitignore"), ".notegit/\n!.notegit/\n").expect("write");

        assert!(!gitignore_ignores_notegit(dir.path()).expect("status"));

        ensure_gitignore_ignores_notegit(dir.path()).expect("ensure");
        let content = std::fs::read_to_string(dir.path().join(".gitignore")).expect("read");
        assert_eq!(content.lines().last(), Some(".notegit/"));
        assert!(gitignore_ignores_notegit(dir.path()).expect("status"));
    }

    #[test]
    fn deveignore_only_workspace_allows_identity_bootstrap() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo_id = uuid::Uuid::new_v4();
        std::fs::write(dir.path().join(".deveignore"), "ignored/*.md\n").expect("deveignore");

        ensure_repo_identity_marker(dir.path(), repo_id, "default").expect("identity marker");

        validate_repo_identity_marker(dir.path(), repo_id).expect("identity marker validation");
    }

    #[test]
    fn git_internal_workspace_allows_identity_bootstrap() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo_id = uuid::Uuid::new_v4();
        let object = dir.path().join(".git/objects/x");
        std::fs::create_dir_all(object.parent().expect("parent")).expect("mkdir");
        std::fs::write(&object, "git object").expect("git object");

        ensure_repo_identity_marker(dir.path(), repo_id, "default").expect("identity marker");

        validate_repo_identity_marker(dir.path(), repo_id).expect("identity marker validation");
    }

    #[test]
    fn deveignore_does_not_hide_external_workspace_content_from_identity_gate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo_id = uuid::Uuid::new_v4();
        std::fs::write(dir.path().join(".deveignore"), "foreign.md\n").expect("deveignore");
        std::fs::write(dir.path().join("foreign.md"), "foreign").expect("foreign");

        let err = ensure_repo_identity_marker(dir.path(), repo_id, "default")
            .expect_err("foreign content must still block identity bootstrap");

        assert!(err.to_string().contains("non-internal content"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_gitignore_fails_closed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outside = dir.path().join("outside");
        std::fs::write(&outside, "# outside\n").expect("outside");
        symlink(&outside, dir.path().join(".gitignore")).expect("symlink");

        let err = ensure_gitignore_ignores_notegit(dir.path())
            .expect_err("symlinked .gitignore must fail closed");

        assert!(
            err.to_string()
                .contains("refusing to follow symlinked .gitignore")
        );
    }
}
