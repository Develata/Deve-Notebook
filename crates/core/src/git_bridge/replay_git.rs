//! plan_ref:
//!   - 03_storage#git-ecosystem-coexistence
//!   - 05_diff_logic#git-mirror-lifecycle
//!
//! Git index and tree primitives for projection replay.

use super::error::{GitReplayError, GitReplayResult};
use super::git_cmd;
use crate::source_control::{ChangeStatus, CommitFileDiff};
use crate::utils::{notegit, path::to_forward_slash};
use std::path::Path;

pub(super) fn read_parent_tree(
    repo_root: &Path,
    index_path: &Path,
    parent_git: Option<&str>,
) -> GitReplayResult<()> {
    let envs = [("GIT_INDEX_FILE", index_path)];
    match parent_git {
        Some(parent) => git_cmd::run_env(repo_root, &["read-tree", parent], &envs)?,
        None => git_cmd::run_env(repo_root, &["read-tree", "--empty"], &envs)?,
    };
    Ok(())
}

pub(super) fn apply_diff_to_index(
    repo_root: &Path,
    index_path: &Path,
    diff: &CommitFileDiff,
) -> GitReplayResult<()> {
    match diff.status {
        ChangeStatus::Deleted => remove_path_from_index(repo_root, index_path, &diff.path),
        ChangeStatus::Renamed => {
            if let Some(previous_path) = diff.previous_path.as_deref()
                && to_forward_slash(previous_path) != to_forward_slash(&diff.path)
            {
                remove_path_from_index(repo_root, index_path, previous_path)?;
            }
            add_blob_to_index(
                repo_root,
                index_path,
                &diff.path,
                diff.new_content.as_bytes(),
            )
        }
        ChangeStatus::Added | ChangeStatus::Modified => add_blob_to_index(
            repo_root,
            index_path,
            &diff.path,
            diff.new_content.as_bytes(),
        ),
    }
}

pub(super) fn add_gitignore_to_index(repo_root: &Path, index_path: &Path) -> GitReplayResult<()> {
    let content = std::fs::read(notegit::gitignore_path(repo_root)).map_err(|err| {
        GitReplayError::ReadGitignore {
            message: err.to_string(),
        }
    })?;
    add_blob_to_index(repo_root, index_path, ".gitignore", &content)
}

pub(super) fn add_blob_to_index(
    repo_root: &Path,
    index_path: &Path,
    path: &str,
    content: &[u8],
) -> GitReplayResult<()> {
    let path = validate_mirror_path(path)?;
    let blob = git_cmd::run_stdin(repo_root, &["hash-object", "-w", "--stdin"], content)?
        .trim()
        .to_string();
    git_cmd::run_env(
        repo_root,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            "100644",
            &blob,
            &path,
        ],
        &[("GIT_INDEX_FILE", index_path)],
    )?;
    Ok(())
}

fn remove_path_from_index(repo_root: &Path, index_path: &Path, path: &str) -> GitReplayResult<()> {
    let path = validate_mirror_path(path)?;
    git_cmd::run_env(
        repo_root,
        &["update-index", "--force-remove", "--", &path],
        &[("GIT_INDEX_FILE", index_path)],
    )?;
    Ok(())
}

fn validate_mirror_path(path: &str) -> GitReplayResult<String> {
    let path = to_forward_slash(path);
    if path.is_empty()
        || path.starts_with('/')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || notegit::is_internal_repo_path(&path)
    {
        return Err(GitReplayError::UnsafeProjectionPath { path });
    }
    Ok(path)
}

pub(super) fn commit_tree(
    repo_root: &Path,
    tree: &str,
    parent_git: Option<&str>,
    message: &str,
) -> GitReplayResult<String> {
    let commit = match parent_git {
        Some(parent) => git_cmd::run(
            repo_root,
            &["commit-tree", tree, "-p", parent, "-m", message],
        )?,
        None => git_cmd::run(repo_root, &["commit-tree", tree, "-m", message])?,
    };
    Ok(commit.trim().to_string())
}

pub(super) fn update_head(
    repo_root: &Path,
    git_commit: &str,
    old_parent: Option<&str>,
) -> GitReplayResult<()> {
    match old_parent {
        Some(parent) => git_cmd::run(repo_root, &["update-ref", "HEAD", git_commit, parent]),
        None => git_cmd::run(repo_root, &["update-ref", "HEAD", git_commit]),
    }?;
    Ok(())
}

pub(super) fn sync_main_index_to_head(repo_root: &Path) -> GitReplayResult<()> {
    git_cmd::run(repo_root, &["read-tree", "--reset", "HEAD"])?;
    Ok(())
}

pub(super) fn ensure_git_commit_exists(repo_root: &Path, git_commit: &str) -> GitReplayResult<()> {
    let commit_object = format!("{git_commit}^{{commit}}");
    git_cmd::run(repo_root, &["cat-file", "-e", &commit_object])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_mirror_path;

    #[test]
    fn validate_mirror_path_rejects_internal_and_escape_paths() {
        for path in ["", "/abs.md", "../x.md", "a/../x.md", ".notegit/state"] {
            let err = validate_mirror_path(path).expect_err("unsafe path rejected");
            assert!(
                err.to_string()
                    .starts_with("Git mirror refuses unsafe projection path: ")
            );
        }
    }

    #[test]
    fn validate_mirror_path_normalizes_safe_paths() {
        assert_eq!(
            validate_mirror_path("dir\\note.md").expect("safe path"),
            "dir/note.md"
        );
    }
}
