//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!
//! Small Git command wrapper for the mirror bridge.

use crate::utils::path::to_forward_slash;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub(super) fn run(repo_root: &Path, args: &[&str]) -> std::result::Result<String, String> {
    let output = base_command(repo_root, args)
        .output()
        .map_err(|err| format!("failed to run git {}: {err}", args.join(" ")))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    Err(git_error(args, &output))
}

pub(super) fn run_env(
    repo_root: &Path,
    args: &[&str],
    envs: &[(&str, &Path)],
) -> std::result::Result<String, String> {
    let mut command = base_command(repo_root, args);
    for (key, value) in envs {
        command.env(key, value);
    }
    let output = command
        .output()
        .map_err(|err| format!("failed to run git {}: {err}", args.join(" ")))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    Err(git_error(args, &output))
}

pub(super) fn run_stdin(
    repo_root: &Path,
    args: &[&str],
    stdin: &[u8],
) -> std::result::Result<String, String> {
    let mut child = base_command(repo_root, args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("failed to run git {}: {err}", args.join(" ")))?;
    child
        .stdin
        .take()
        .ok_or_else(|| format!("failed to open stdin for git {}", args.join(" ")))?
        .write_all(stdin)
        .map_err(|err| format!("failed to write stdin for git {}: {err}", args.join(" ")))?;
    let output = child
        .wait_with_output()
        .map_err(|err| format!("failed to wait for git {}: {err}", args.join(" ")))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    Err(git_error(args, &output))
}

pub(super) fn run_z_paths(
    repo_root: &Path,
    args: &[&str],
) -> std::result::Result<Vec<String>, String> {
    Ok(run_z_fields(repo_root, args)?
        .into_iter()
        .map(|path| to_forward_slash(&path))
        .collect())
}

pub(super) fn run_z_fields(
    repo_root: &Path,
    args: &[&str],
) -> std::result::Result<Vec<String>, String> {
    let output = base_command(repo_root, args)
        .output()
        .map_err(|err| format!("failed to run git {}: {err}", args.join(" ")))?;
    if !output.status.success() {
        return Err(git_error(args, &output));
    }
    let mut fields = Vec::new();
    for raw in output.stdout.split(|byte| *byte == 0) {
        if raw.is_empty() {
            continue;
        }
        let field = std::str::from_utf8(raw)
            .map_err(|err| format!("git {} returned non-UTF-8 field: {err}", args.join(" ")))?;
        fields.push(field.to_string());
    }
    Ok(fields)
}

pub(super) fn has_staged_changes(repo_root: &Path) -> std::result::Result<bool, String> {
    let args = ["diff", "--cached", "--quiet"];
    let output = base_command(repo_root, &args)
        .output()
        .map_err(|err| format!("failed to run git diff --cached --quiet: {err}"))?;
    match output.status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => Err(git_error(&args, &output)),
    }
}

pub(super) fn current_head(repo_root: &Path) -> std::result::Result<Option<String>, String> {
    let args = ["rev-parse", "--verify", "HEAD"];
    let output = base_command(repo_root, &args)
        .output()
        .map_err(|err| format!("failed to run git rev-parse --verify HEAD: {err}"))?;
    if output.status.success() {
        return Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
        ));
    }
    Ok(None)
}

fn base_command(repo_root: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");
    command.arg("-C").arg(repo_root).args(args);
    command
}

fn git_error(args: &[&str], output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() { stderr } else { stdout };
    if detail.is_empty() {
        format!(
            "git {} failed with status {}",
            args.join(" "),
            output.status
        )
    } else {
        format!(
            "git {} failed (status {}): {detail}",
            args.join(" "),
            output.status
        )
    }
}
