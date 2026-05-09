//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!
//! Read-only Git ecosystem mirror inspection.

use super::error::{GitMirrorStatusError, GitMirrorStatusResult};
use crate::utils::notegit;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitMirrorState {
    Disabled,
    Ready,
    ProtectionMissing,
    OutOfSync,
}

impl GitMirrorState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Ready => "ready",
            Self::ProtectionMissing => "protection_missing",
            Self::OutOfSync => "out_of_sync",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitMetadataKind {
    Missing,
    Directory,
    File,
    Other,
}

impl GitMetadataKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Directory => "directory",
            Self::File => "file",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitMirrorStatus {
    pub repo_root: PathBuf,
    pub notegit_present: bool,
    pub git_metadata_kind: GitMetadataKind,
    pub gitignore_protects_notegit: bool,
    pub state: GitMirrorState,
    pub reason: Option<String>,
}

pub fn inspect_repo_root(repo_root: &Path) -> GitMirrorStatusResult<GitMirrorStatus> {
    let notegit_present = notegit::repo_dir(repo_root).try_exists().map_err(|err| {
        GitMirrorStatusError::NotegitPresence {
            message: err.to_string(),
        }
    })?;
    let git_metadata_kind = classify_git_metadata(repo_root)?;
    let gitignore_protects_notegit =
        notegit::gitignore_ignores_notegit(repo_root).map_err(|err| {
            GitMirrorStatusError::GitignoreProtection {
                message: err.to_string(),
            }
        })?;
    let (state, reason) = classify_state(git_metadata_kind, gitignore_protects_notegit);

    Ok(GitMirrorStatus {
        repo_root: repo_root.to_path_buf(),
        notegit_present,
        git_metadata_kind,
        gitignore_protects_notegit,
        state,
        reason,
    })
}

fn classify_git_metadata(repo_root: &Path) -> GitMirrorStatusResult<GitMetadataKind> {
    let path = repo_root.join(notegit::GIT_DIR);
    if !path
        .try_exists()
        .map_err(|err| GitMirrorStatusError::GitMetadataPresence {
            message: err.to_string(),
        })?
    {
        return Ok(GitMetadataKind::Missing);
    }
    let meta = std::fs::symlink_metadata(path).map_err(|err| {
        GitMirrorStatusError::GitMetadataInspect {
            message: err.to_string(),
        }
    })?;
    if meta.is_dir() {
        return Ok(GitMetadataKind::Directory);
    }
    if meta.is_file() {
        return Ok(GitMetadataKind::File);
    }
    Ok(GitMetadataKind::Other)
}

fn classify_state(
    git_metadata_kind: GitMetadataKind,
    gitignore_protects_notegit: bool,
) -> (GitMirrorState, Option<String>) {
    if git_metadata_kind == GitMetadataKind::Missing {
        return (GitMirrorState::Disabled, None);
    }
    if !gitignore_protects_notegit {
        return (
            GitMirrorState::ProtectionMissing,
            Some("repo-local .gitignore does not ignore .notegit/".to_string()),
        );
    }
    (GitMirrorState::Ready, None)
}

#[cfg(test)]
mod tests {
    use super::{GitMetadataKind, GitMirrorState, inspect_repo_root};

    #[test]
    fn missing_git_metadata_is_disabled() {
        let dir = tempfile::tempdir().expect("tempdir");
        crate::utils::notegit::ensure_gitignore_ignores_notegit(dir.path()).expect("gitignore");

        let status = inspect_repo_root(dir.path()).expect("inspect");

        assert_eq!(status.git_metadata_kind, GitMetadataKind::Missing);
        assert_eq!(status.state, GitMirrorState::Disabled);
        assert!(status.gitignore_protects_notegit);
    }

    #[test]
    fn git_dir_without_notegit_ignore_is_protection_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".git")).expect("mkdir git");

        let status = inspect_repo_root(dir.path()).expect("inspect");

        assert_eq!(status.git_metadata_kind, GitMetadataKind::Directory);
        assert_eq!(status.state, GitMirrorState::ProtectionMissing);
        assert!(!status.gitignore_protects_notegit);
    }

    #[test]
    fn git_dir_with_notegit_ignore_is_ready() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".git")).expect("mkdir git");
        crate::utils::notegit::ensure_gitignore_ignores_notegit(dir.path()).expect("gitignore");

        let status = inspect_repo_root(dir.path()).expect("inspect");

        assert_eq!(status.git_metadata_kind, GitMetadataKind::Directory);
        assert_eq!(status.state, GitMirrorState::Ready);
        assert!(status.gitignore_protects_notegit);
    }
}
