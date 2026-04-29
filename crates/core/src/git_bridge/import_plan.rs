//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!   - 12_commands#cli-commands
//!
//! Read-only Git import planning. This module observes external Git/worktree
//! changes and never writes Deve ledger, pending_fs, staging, or `.notegit`.

use super::git_cmd;
use super::preflight::{ensure_git_worktree, ensure_notegit_is_not_tracked};
use super::status::{GitMirrorState, inspect_repo_root};
use crate::source_control::ChangeStatus;
use crate::utils::{notegit, path::to_forward_slash};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitImportPlan {
    pub repo_root: PathBuf,
    pub entries: Vec<GitImportPlanEntry>,
    pub blockers: Vec<GitImportPlanBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitImportPlanEntry {
    pub path: String,
    #[serde(default)]
    pub previous_path: Option<String>,
    pub status: ChangeStatus,
    pub git_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitImportPlanBlocker {
    pub path: String,
    pub reason: String,
}

pub fn plan_import(repo_root: &Path) -> Result<GitImportPlan> {
    ensure_import_ready(repo_root)?;
    let fields = git_cmd::run_z_fields(
        repo_root,
        &["diff", "--name-status", "-z", "-M", "HEAD", "--"],
    )
    .map_err(|err| anyhow!(err))?;
    let mut plan = GitImportPlan {
        repo_root: repo_root.to_path_buf(),
        entries: Vec::new(),
        blockers: Vec::new(),
    };
    parse_name_status_fields(&mut plan, &fields);
    add_untracked_entries(&mut plan, repo_root)?;
    plan.entries.sort_by(|left, right| {
        (&left.path, &left.previous_path).cmp(&(&right.path, &right.previous_path))
    });
    plan.blockers
        .sort_by(|left, right| (&left.path, &left.reason).cmp(&(&right.path, &right.reason)));
    Ok(plan)
}

fn ensure_import_ready(repo_root: &Path) -> Result<()> {
    let status = inspect_repo_root(repo_root)?;
    if status.state != GitMirrorState::Ready {
        let reason = status.reason.unwrap_or_else(|| {
            format!(
                "state={} git={}",
                status.state.as_str(),
                status.git_metadata_kind.as_str()
            )
        });
        return Err(anyhow!(
            "Git import dry-run requires ready Git mirror: {reason}"
        ));
    }
    ensure_git_worktree(repo_root).map_err(|err| anyhow!(err))?;
    ensure_notegit_is_not_tracked(repo_root).map_err(|err| anyhow!(err))?;
    if git_cmd::current_head(repo_root)
        .map_err(|err| anyhow!(err))?
        .is_none()
    {
        return Err(anyhow!("Git import dry-run requires Git HEAD"));
    }
    Ok(())
}

fn parse_name_status_fields(plan: &mut GitImportPlan, fields: &[String]) {
    let mut index = 0;
    while index < fields.len() {
        let git_status = fields[index].clone();
        index += 1;
        let status_code = git_status.chars().next().unwrap_or_default();
        match status_code {
            'A' | 'M' | 'D' => {
                let Some(path) = take_field(fields, &mut index, &git_status, plan) else {
                    break;
                };
                push_regular_entry(plan, path, git_status, status_from_code(status_code));
            }
            'R' => {
                let Some(previous_path) = take_field(fields, &mut index, &git_status, plan) else {
                    break;
                };
                let Some(path) = take_field(fields, &mut index, &git_status, plan) else {
                    break;
                };
                push_rename_entry(plan, previous_path, path, git_status);
            }
            'C' => {
                let path = take_field(fields, &mut index, &git_status, plan)
                    .unwrap_or_else(|| "-".to_string());
                let _ = take_field(fields, &mut index, &git_status, plan);
                push_blocker(
                    plan,
                    path,
                    "copy changes are not supported by Git import dry-run yet",
                );
            }
            _ => {
                let path = take_field(fields, &mut index, &git_status, plan)
                    .unwrap_or_else(|| "-".to_string());
                push_blocker(plan, path, format!("unsupported Git status {git_status}"));
            }
        }
    }
}

fn add_untracked_entries(plan: &mut GitImportPlan, repo_root: &Path) -> Result<()> {
    let paths = git_cmd::run_z_fields(
        repo_root,
        &["ls-files", "-o", "--exclude-standard", "-z", "--"],
    )
    .map_err(|err| anyhow!(err))?;
    for path in paths {
        push_regular_entry(plan, path, "??".to_string(), ChangeStatus::Added);
    }
    Ok(())
}

fn take_field(
    fields: &[String],
    index: &mut usize,
    git_status: &str,
    plan: &mut GitImportPlan,
) -> Option<String> {
    if let Some(field) = fields.get(*index) {
        *index += 1;
        return Some(field.clone());
    }
    push_blocker(
        plan,
        "-",
        format!("malformed Git name-status record for {git_status}"),
    );
    None
}

fn status_from_code(status_code: char) -> ChangeStatus {
    match status_code {
        'A' => ChangeStatus::Added,
        'D' => ChangeStatus::Deleted,
        _ => ChangeStatus::Modified,
    }
}

fn push_regular_entry(
    plan: &mut GitImportPlan,
    path: String,
    git_status: String,
    status: ChangeStatus,
) {
    match validate_import_path(&path) {
        Ok(path) => plan.entries.push(GitImportPlanEntry {
            path,
            previous_path: None,
            status,
            git_status,
        }),
        Err(reason) => push_blocker(plan, normalized_display_path(&path), reason),
    }
}

fn push_rename_entry(
    plan: &mut GitImportPlan,
    previous_path: String,
    path: String,
    git_status: String,
) {
    let previous = validate_import_path(&previous_path);
    let current = validate_import_path(&path);
    match (previous, current) {
        (Ok(previous_path), Ok(path)) => plan.entries.push(GitImportPlanEntry {
            path,
            previous_path: Some(previous_path),
            status: ChangeStatus::Renamed,
            git_status,
        }),
        (Err(reason), _) => push_blocker(plan, normalized_display_path(&previous_path), reason),
        (_, Err(reason)) => push_blocker(plan, normalized_display_path(&path), reason),
    }
}

fn validate_import_path(path: &str) -> std::result::Result<String, String> {
    let path = to_forward_slash(path);
    if path.is_empty()
        || path.starts_with('/')
        || has_windows_drive_prefix(&path)
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || notegit::is_internal_repo_path(&path)
    {
        return Err(format!("Git import refuses unsafe path: {path}"));
    }
    Ok(path)
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn normalized_display_path(path: &str) -> String {
    let path = to_forward_slash(path);
    if path.is_empty() {
        "-".to_string()
    } else {
        path
    }
}

fn push_blocker(plan: &mut GitImportPlan, path: impl Into<String>, reason: impl Into<String>) {
    plan.blockers.push(GitImportPlanBlocker {
        path: path.into(),
        reason: reason.into(),
    });
}

#[cfg(test)]
mod tests {
    use super::{GitImportPlan, plan_import, validate_import_path};
    use crate::source_control::ChangeStatus;
    use std::path::Path;
    use std::process::Command;

    fn git(path: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    fn init_git_repo(path: &Path) {
        std::fs::create_dir_all(path).expect("repo dir");
        git(path, &["init"]);
        git(path, &["config", "user.email", "deve@example.invalid"]);
        git(path, &["config", "user.name", "Deve Test"]);
        crate::utils::notegit::ensure_gitignore_ignores_notegit(path).expect("gitignore");
    }

    fn commit_baseline(path: &Path) {
        std::fs::write(path.join("delete.md"), "delete\n").expect("delete seed");
        std::fs::write(path.join("mod.md"), "old\n").expect("mod seed");
        std::fs::write(path.join("rename_from.md"), "rename\n").expect("rename seed");
        git(path, &["add", "."]);
        git(path, &["commit", "--no-gpg-sign", "-m", "baseline"]);
    }

    fn entry<'a>(plan: &'a GitImportPlan, path: &str) -> &'a super::GitImportPlanEntry {
        plan.entries
            .iter()
            .find(|entry| entry.path == path)
            .unwrap_or_else(|| panic!("missing entry {path}: {:?}", plan.entries))
    }

    #[test]
    fn plan_import_reports_tracked_and_untracked_changes() {
        let dir = tempfile::tempdir().expect("tempdir");
        init_git_repo(dir.path());
        commit_baseline(dir.path());

        std::fs::write(dir.path().join("mod.md"), "new\n").expect("modify");
        std::fs::remove_file(dir.path().join("delete.md")).expect("delete");
        git(dir.path(), &["mv", "rename_from.md", "rename_to.md"]);
        std::fs::write(dir.path().join("new.md"), "new file\n").expect("new");

        let plan = plan_import(dir.path()).expect("plan import");

        assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
        assert_eq!(plan.entries.len(), 4);
        assert_eq!(entry(&plan, "delete.md").status, ChangeStatus::Deleted);
        assert_eq!(entry(&plan, "mod.md").status, ChangeStatus::Modified);
        assert_eq!(entry(&plan, "new.md").status, ChangeStatus::Added);
        let renamed = entry(&plan, "rename_to.md");
        assert_eq!(renamed.status, ChangeStatus::Renamed);
        assert_eq!(renamed.previous_path.as_deref(), Some("rename_from.md"));
        assert!(renamed.git_status.starts_with('R'));
    }

    #[test]
    fn plan_import_requires_git_head() {
        let dir = tempfile::tempdir().expect("tempdir");
        init_git_repo(dir.path());

        let err = plan_import(dir.path()).expect_err("missing HEAD must fail closed");

        assert!(err.to_string().contains("requires Git HEAD"));
    }

    #[test]
    fn validate_import_path_rejects_internal_and_escape_paths() {
        assert_eq!(
            validate_import_path("notes/file.md").expect("valid"),
            "notes/file.md"
        );
        for path in [
            "",
            "/abs.md",
            "C:/abs.md",
            "notes//file.md",
            "notes/../file.md",
            ".git/config",
            "notes/.notegit/state",
        ] {
            let err = validate_import_path(path).expect_err("unsafe path");
            assert!(err.contains("unsafe path"), "{err}");
        }
    }
}
