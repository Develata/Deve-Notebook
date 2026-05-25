//! plan_ref:
//!   - 03_storage#git-ecosystem-coexistence
//!   - 05_diff_logic#git-mirror-lifecycle
//!
//! Small Git command wrapper for the mirror bridge.

use super::error::{GitCommandError, GitCommandResult};
use crate::utils::path::to_forward_slash;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub(super) fn run(repo_root: &Path, args: &[&str]) -> GitCommandResult<String> {
    let output = base_command(repo_root, args)
        .output()
        .map_err(|err| GitCommandError::Spawn {
            args: args_label(args),
            message: err.to_string(),
        })?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    Err(git_error(args, &output))
}

pub(super) fn run_env(
    repo_root: &Path,
    args: &[&str],
    envs: &[(&str, &Path)],
) -> GitCommandResult<String> {
    let mut command = base_command(repo_root, args);
    for (key, value) in envs {
        command.env(key, value);
    }
    let output = command.output().map_err(|err| GitCommandError::Spawn {
        args: args_label(args),
        message: err.to_string(),
    })?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    Err(git_error(args, &output))
}

pub(super) fn run_stdin(repo_root: &Path, args: &[&str], stdin: &[u8]) -> GitCommandResult<String> {
    let mut child = base_command(repo_root, args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| GitCommandError::Spawn {
            args: args_label(args),
            message: err.to_string(),
        })?;
    child
        .stdin
        .take()
        .ok_or_else(|| GitCommandError::MissingStdin {
            args: args_label(args),
        })?
        .write_all(stdin)
        .map_err(|err| GitCommandError::StdinWrite {
            args: args_label(args),
            message: err.to_string(),
        })?;
    let output = child
        .wait_with_output()
        .map_err(|err| GitCommandError::Wait {
            args: args_label(args),
            message: err.to_string(),
        })?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    Err(git_error(args, &output))
}

pub(super) fn run_z_paths(repo_root: &Path, args: &[&str]) -> GitCommandResult<Vec<String>> {
    Ok(run_z_fields(repo_root, args)?
        .into_iter()
        .map(|path| to_forward_slash(&path))
        .collect())
}

pub(super) fn run_z_fields(repo_root: &Path, args: &[&str]) -> GitCommandResult<Vec<String>> {
    let output = base_command(repo_root, args)
        .output()
        .map_err(|err| GitCommandError::Spawn {
            args: args_label(args),
            message: err.to_string(),
        })?;
    if !output.status.success() {
        return Err(git_error(args, &output));
    }
    let mut fields = Vec::new();
    for raw in output.stdout.split(|byte| *byte == 0) {
        if raw.is_empty() {
            continue;
        }
        let field = std::str::from_utf8(raw).map_err(|err| GitCommandError::NonUtf8Field {
            args: args_label(args),
            message: err.to_string(),
        })?;
        fields.push(field.to_string());
    }
    Ok(fields)
}

pub(super) fn has_staged_changes(repo_root: &Path) -> GitCommandResult<bool> {
    let args = ["diff", "--cached", "--quiet"];
    let output = base_command(repo_root, &args)
        .output()
        .map_err(|err| GitCommandError::Spawn {
            args: args_label(&args),
            message: err.to_string(),
        })?;
    match output.status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => Err(git_error(&args, &output)),
    }
}

pub(super) fn current_head(repo_root: &Path) -> GitCommandResult<Option<String>> {
    let args = ["rev-parse", "--verify", "HEAD"];
    let output = base_command(repo_root, &args)
        .output()
        .map_err(|err| GitCommandError::Spawn {
            args: args_label(&args),
            message: err.to_string(),
        })?;
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

fn args_label(args: &[&str]) -> String {
    args.join(" ")
}

fn git_error(args: &[&str], output: &std::process::Output) -> GitCommandError {
    GitCommandError::status(args, output)
}
