//! plan_ref:
//!   - 05_diff_logic#git-mirror-lifecycle
//!   - 14_commands#cli-commands
//!

use super::{GitMirrorPushBlocker, GitMirrorPushOptions, GitMirrorPushReport};
use crate::git_bridge::error::{GitBridgeError, GitBridgeResult};
use crate::git_bridge::git_cmd;
use std::path::Path;

pub(super) fn resolve_push_target(
    repo_root: &Path,
    options: &GitMirrorPushOptions,
    report: &mut GitMirrorPushReport,
) {
    let branch = match options.branch.as_deref() {
        Some(branch) => validate_push_name(branch, "branch")
            .map(|branch| branch.to_string())
            .map_err(|err| blocker("git_remote", err.to_string())),
        None => current_branch(repo_root).map_err(|err| blocker("git_remote", err.to_string())),
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
            .map_err(|err| blocker("git_remote", err.to_string())),
        None => {
            default_remote(repo_root, &branch).map_err(|err| blocker("git_remote", err.to_string()))
        }
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

fn current_branch(repo_root: &Path) -> GitBridgeResult<String> {
    let branch = git_cmd::run(repo_root, &["branch", "--show-current"])
        .map_err(GitBridgeError::GitCommand)?;
    let branch = branch.trim();
    if branch.is_empty() {
        return Err(GitBridgeError::DetachedHead);
    }
    validate_push_name(branch, "branch").map(|branch| branch.to_string())
}

fn default_remote(repo_root: &Path, branch: &str) -> GitBridgeResult<String> {
    let key = format!("branch.{branch}.remote");
    if let Ok(remote) = git_cmd::run(repo_root, &["config", "--get", &key]) {
        let remote = remote.trim();
        if !remote.is_empty() {
            return validate_push_name(remote, "remote").map(|remote| remote.to_string());
        }
    }
    Ok("origin".to_string())
}

pub(super) fn validate_push_name<'a>(
    value: &'a str,
    label: &'static str,
) -> GitBridgeResult<&'a str> {
    if value.is_empty()
        || value.starts_with('-')
        || value
            .chars()
            .any(|ch| ch.is_ascii_control() || ch.is_ascii_whitespace())
    {
        return Err(GitBridgeError::InvalidPushName {
            label,
            value: value.to_string(),
        });
    }
    Ok(value)
}

pub(super) fn blocker(
    location: impl Into<String>,
    reason: impl Into<String>,
) -> GitMirrorPushBlocker {
    GitMirrorPushBlocker {
        location: location.into(),
        reason: reason.into(),
    }
}
