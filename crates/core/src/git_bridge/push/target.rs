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
            .map_err(|_| push_target_failure_blocker()),
        None => current_branch(repo_root).map_err(|_| push_target_failure_blocker()),
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
            .map_err(|_| push_target_failure_blocker()),
        None => default_remote(repo_root, &branch).map_err(|_| push_target_failure_blocker()),
    };
    let remote = match remote {
        Ok(remote) => remote,
        Err(blocker) => {
            report.blockers.push(blocker);
            return;
        }
    };

    match git_cmd::run(repo_root, &["remote", "get-url", &remote]) {
        Ok(url) => {
            report.remote = Some(remote);
            report.remote_url = sanitize_remote_url(&url);
        }
        Err(_) => report.blockers.push(push_target_failure_blocker()),
    }
}

fn push_target_failure_blocker() -> GitMirrorPushBlocker {
    blocker(
        "git_remote",
        "Git push mirror target is invalid or unavailable; inspect remote and branch configuration",
    )
}

pub(super) fn sanitize_remote_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return None;
    }
    let without_query = trimmed.split(['?', '#']).next().unwrap_or_default();
    if let Some((scheme, _)) = without_query.split_once("://") {
        if !matches!(scheme, "git" | "http" | "https" | "ssh") {
            return None;
        }
        let authority_start = scheme.len() + 3;
        let authority_end = without_query[authority_start..]
            .find('/')
            .map_or(without_query.len(), |offset| authority_start + offset);
        let authority = &without_query[authority_start..authority_end];
        let host = authority
            .rsplit_once('@')
            .map_or(authority, |(_, host)| host);
        let path = &without_query[authority_end..];
        if !safe_remote_authority(host) || path.contains('\\') {
            return None;
        }
        return Some(format!("{scheme}://{host}{path}"));
    }
    let scp = without_query
        .rsplit_once('@')
        .map_or(without_query, |(_, location)| location);
    let (host, path) = scp.split_once(':')?;
    if !safe_remote_hostname(host) || path.is_empty() || path.contains(['\\', ':']) {
        return None;
    }
    Some(format!("{host}:{path}"))
}

fn safe_remote_authority(authority: &str) -> bool {
    if let Some(bracketed) = authority.strip_prefix('[') {
        let Some((address, suffix)) = bracketed.split_once(']') else {
            return false;
        };
        if address.parse::<std::net::Ipv6Addr>().is_err() {
            return false;
        }
        return suffix.is_empty() || valid_numeric_port(suffix.strip_prefix(':'));
    }
    match authority.rsplit_once(':') {
        Some((host, port)) => {
            !host.contains(':') && safe_remote_hostname(host) && valid_numeric_port(Some(port))
        }
        None => safe_remote_hostname(authority),
    }
}

fn valid_numeric_port(port: Option<&str>) -> bool {
    port.is_some_and(|port| !port.is_empty() && port.parse::<u16>().is_ok())
}

fn safe_remote_hostname(host: &str) -> bool {
    !host.is_empty()
        && host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
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
    let valid = match label {
        "branch" => is_valid_branch_name(value),
        "remote" => is_valid_remote_name(value),
        _ => false,
    };
    if !valid {
        return Err(GitBridgeError::InvalidPushName {
            label,
            value: value.to_string(),
        });
    }
    Ok(value)
}

fn is_valid_remote_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.starts_with(['-', '/', '.'])
        && !value.ends_with(['/', '.'])
        && !value.contains("..")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/'))
}

fn is_valid_branch_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && value != "@"
        && !value.starts_with(['-', '/', '.'])
        && !value.ends_with(['/', '.'])
        && !value.contains("..")
        && !value.contains("//")
        && !value.contains("@{")
        && value.split('/').all(|segment| {
            !segment.is_empty()
                && !segment.starts_with('.')
                && !segment.ends_with(".lock")
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        })
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
