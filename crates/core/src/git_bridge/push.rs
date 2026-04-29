//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!   - 12_commands#cli-commands
//!
//! Explicit Git mirror remote publish. This module only pushes the `.git`
//! mirror and never treats Git refs as Deve authority.

use super::git_cmd;
use super::preflight::{
    ensure_git_worktree, ensure_git_worktree_clean, ensure_notegit_is_not_tracked,
    ensure_source_control_clean,
};
use super::status::{GitMirrorState, inspect_repo_root};
use super::store::{GitMirrorCommitState, GitMirrorRecord, list_records};
use anyhow::Result;
use redb::Database;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitMirrorPushOptions {
    pub remote: Option<String>,
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitMirrorPushReport {
    pub remote: Option<String>,
    pub branch: Option<String>,
    pub remote_url: Option<String>,
    pub head: Option<String>,
    pub pushed: bool,
    pub blockers: Vec<GitMirrorPushBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitMirrorPushBlocker {
    pub location: String,
    pub reason: String,
}

pub fn push_mirror(
    db: &Database,
    repo_root: &Path,
    options: GitMirrorPushOptions,
) -> Result<GitMirrorPushReport> {
    let mut report = GitMirrorPushReport::default();
    let status = inspect_repo_root(repo_root)?;
    if status.state != GitMirrorState::Ready {
        report.blockers.push(blocker(
            "mirror_not_ready",
            status.reason.unwrap_or_else(|| {
                format!(
                    "Git push mirror requires ready Git mirror: state={} git={}",
                    status.state.as_str(),
                    status.git_metadata_kind.as_str()
                )
            }),
        ));
        return Ok(report);
    }

    collect_preflight_blockers(db, repo_root, &mut report)?;
    let records = list_records(db)?;
    collect_mapping_blockers(&records, &mut report);
    resolve_push_target(repo_root, &options, &mut report);

    if !report.blockers.is_empty() {
        return Ok(report);
    }

    let remote = report.remote.as_deref().expect("remote resolved");
    let branch = report.branch.as_deref().expect("branch resolved");
    match git_cmd::run(repo_root, &["push", remote, branch]) {
        Ok(_) => {
            report.pushed = true;
        }
        Err(reason) => report.blockers.push(blocker("git_command", reason)),
    }
    Ok(report)
}

fn collect_preflight_blockers(
    db: &Database,
    repo_root: &Path,
    report: &mut GitMirrorPushReport,
) -> Result<()> {
    for (location, result) in [
        ("git_worktree", ensure_git_worktree(repo_root)),
        (
            "notegit_protection",
            ensure_notegit_is_not_tracked(repo_root),
        ),
        ("deve_source_control", ensure_source_control_clean(db)),
        ("git_worktree", ensure_git_worktree_clean(repo_root)),
    ] {
        if let Err(reason) = result {
            report.blockers.push(blocker(location, reason));
        }
    }

    match git_cmd::current_head(repo_root) {
        Ok(Some(head)) => report.head = Some(head),
        Ok(None) => report.blockers.push(blocker(
            "git_history_mapping",
            "Git push mirror requires Git HEAD; run `deve_cli git export` first",
        )),
        Err(reason) => report.blockers.push(blocker("git_worktree", reason)),
    }
    Ok(())
}

fn collect_mapping_blockers(records: &[GitMirrorRecord], report: &mut GitMirrorPushReport) {
    let queued = records
        .iter()
        .filter(|record| record.state == GitMirrorCommitState::Queued)
        .count();
    let out_of_sync = records
        .iter()
        .filter(|record| record.state == GitMirrorCommitState::OutOfSync)
        .count();
    if queued > 0 || out_of_sync > 0 {
        report.blockers.push(blocker(
            "git_history_mapping",
            format!(
                "Git push mirror refuses unpublished mirror records: queued={queued} out_of_sync={out_of_sync}; run `deve_cli git export` or repair first"
            ),
        ));
    }

    let Some(head) = report.head.as_deref() else {
        return;
    };
    let Some(latest) = records
        .iter()
        .filter(|record| record.state == GitMirrorCommitState::Committed)
        .max_by_key(|record| record.ledger_seq)
    else {
        report.blockers.push(blocker(
            "git_history_mapping",
            "Git push mirror refuses Git history without Deve mirror mapping; run `deve_cli git export` first",
        ));
        return;
    };
    if latest.git_commit_id.as_deref() != Some(head) {
        report.blockers.push(blocker(
            "git_history_mapping",
            format!(
                "Git push mirror refuses stale Git HEAD {head}; latest mirrored Deve commit {} maps to {}",
                latest.deve_commit_id,
                latest.git_commit_id.as_deref().unwrap_or("-")
            ),
        ));
    }
}

fn resolve_push_target(
    repo_root: &Path,
    options: &GitMirrorPushOptions,
    report: &mut GitMirrorPushReport,
) {
    let branch = match options.branch.as_deref() {
        Some(branch) => validate_push_name(branch, "branch")
            .map(|branch| branch.to_string())
            .map_err(|reason| blocker("git_remote", reason)),
        None => current_branch(repo_root).map_err(|reason| blocker("git_remote", reason)),
    };
    let branch = match branch {
        Ok(branch) => {
            report.branch = Some(branch.clone());
            branch
        }
        Err(blocker) => {
            report.blockers.push(blocker);
            return;
        }
    };

    let remote = match options.remote.as_deref() {
        Some(remote) => validate_push_name(remote, "remote")
            .map(|remote| remote.to_string())
            .map_err(|reason| blocker("git_remote", reason)),
        None => default_remote(repo_root, &branch).map_err(|reason| blocker("git_remote", reason)),
    };
    let remote = match remote {
        Ok(remote) => {
            report.remote = Some(remote.clone());
            remote
        }
        Err(blocker) => {
            report.blockers.push(blocker);
            return;
        }
    };

    match git_cmd::run(repo_root, &["remote", "get-url", &remote]) {
        Ok(url) => report.remote_url = Some(url.trim().to_string()),
        Err(reason) => report.blockers.push(blocker("git_remote", reason)),
    }
}

fn current_branch(repo_root: &Path) -> std::result::Result<String, String> {
    let branch = git_cmd::run(repo_root, &["branch", "--show-current"])?;
    let branch = branch.trim();
    if branch.is_empty() {
        return Err(
            "Git push mirror requires a named branch; detached HEAD needs --branch".to_string(),
        );
    }
    validate_push_name(branch, "branch").map(|branch| branch.to_string())
}

fn default_remote(repo_root: &Path, branch: &str) -> std::result::Result<String, String> {
    let key = format!("branch.{branch}.remote");
    if let Ok(remote) = git_cmd::run(repo_root, &["config", "--get", &key]) {
        let remote = remote.trim();
        if !remote.is_empty() {
            return validate_push_name(remote, "remote").map(|remote| remote.to_string());
        }
    }
    Ok("origin".to_string())
}

fn validate_push_name<'a>(value: &'a str, label: &str) -> std::result::Result<&'a str, String> {
    if value.is_empty()
        || value.starts_with('-')
        || value
            .chars()
            .any(|ch| ch.is_ascii_control() || ch.is_ascii_whitespace())
    {
        return Err(format!(
            "Git push mirror refuses invalid {label}: {value:?}"
        ));
    }
    Ok(value)
}

fn blocker(location: impl Into<String>, reason: impl Into<String>) -> GitMirrorPushBlocker {
    GitMirrorPushBlocker {
        location: location.into(),
        reason: reason.into(),
    }
}

#[cfg(test)]
#[path = "push_test.rs"]
mod push_test;
