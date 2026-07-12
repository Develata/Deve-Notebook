//! plan_ref:
//!   - 03_storage/index#git-ecosystem-coexistence
//!   - 05_diff_logic#git-mirror-lifecycle
//!
//! Small Git command wrapper for the mirror bridge.

use super::error::{GitCommandError, GitCommandResult};
use crate::utils::path::to_forward_slash;
use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const DEVE_GIT_EXECUTABLE_ENV: &str = "DEVE_GIT_EXECUTABLE";
pub const DEVE_GIT_EXECUTABLE_UNAVAILABLE_ENV: &str = "DEVE_GIT_EXECUTABLE_UNAVAILABLE";

pub(super) fn run(repo_root: &Path, args: &[&str]) -> GitCommandResult<String> {
    let output = base_command(repo_root, args)?
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
    let mut command = base_command(repo_root, args)?;
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
    let mut child = base_command(repo_root, args)?
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
    let output = base_command(repo_root, args)?
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
    let output =
        base_command(repo_root, &args)?
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
    let output =
        base_command(repo_root, &args)?
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

fn base_command(repo_root: &Path, args: &[&str]) -> GitCommandResult<Command> {
    let executable = resolve_git_executable(
        std::env::var_os(DEVE_GIT_EXECUTABLE_ENV).as_deref(),
        std::env::var_os(DEVE_GIT_EXECUTABLE_UNAVAILABLE_ENV).as_deref(),
    )?;
    let mut command = Command::new(executable);
    command.arg("-C").arg(repo_root).args(args);
    Ok(command)
}

fn resolve_git_executable(
    configured: Option<&OsStr>,
    unavailable: Option<&OsStr>,
) -> GitCommandResult<PathBuf> {
    let unavailable = match unavailable {
        None => false,
        Some(value) if value == OsStr::new("1") => true,
        Some(value) => {
            return Err(GitCommandError::InvalidExecutable {
                message: format!(
                    "{DEVE_GIT_EXECUTABLE_UNAVAILABLE_ENV} must be exactly 1, got {value:?}"
                ),
            });
        }
    };
    if unavailable {
        if configured.is_some() {
            return Err(GitCommandError::InvalidExecutable {
                message: "trusted path and unavailable marker cannot both be set".to_string(),
            });
        }
        return Err(GitCommandError::InvalidExecutable {
            message: "trusted Desktop host reported Git unavailable".to_string(),
        });
    }
    let Some(configured) = configured else {
        return Ok(PathBuf::from("git"));
    };
    let configured = Path::new(configured);
    if !configured.is_absolute() {
        return Err(GitCommandError::InvalidExecutable {
            message: format!("configured path is not absolute: {configured:?}"),
        });
    }
    let canonical =
        std::fs::canonicalize(configured).map_err(|error| GitCommandError::InvalidExecutable {
            message: format!("cannot canonicalize {configured:?}: {error}"),
        })?;
    let metadata = canonical
        .metadata()
        .map_err(|error| GitCommandError::InvalidExecutable {
            message: format!("cannot inspect {canonical:?}: {error}"),
        })?;
    if !metadata.is_file() {
        return Err(GitCommandError::InvalidExecutable {
            message: format!("configured path is not an ordinary file: {canonical:?}"),
        });
    }
    Ok(canonical)
}

fn args_label(args: &[&str]) -> String {
    args.join(" ")
}

fn git_error(args: &[&str], output: &std::process::Output) -> GitCommandError {
    GitCommandError::status(args, output)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn unset_executable_keeps_cli_path_resolution() {
        assert_eq!(resolve_git_executable(None, None), Ok(PathBuf::from("git")));
    }

    #[test]
    fn desktop_unavailable_marker_prevents_cli_path_fallback() {
        assert!(matches!(
            resolve_git_executable(None, Some(OsStr::new("1"))),
            Err(GitCommandError::InvalidExecutable { .. })
        ));
        assert!(matches!(
            resolve_git_executable(Some(OsStr::new("C:\\git.exe")), Some(OsStr::new("1"))),
            Err(GitCommandError::InvalidExecutable { .. })
        ));
        assert!(matches!(
            resolve_git_executable(None, Some(OsStr::new("true"))),
            Err(GitCommandError::InvalidExecutable { .. })
        ));
    }

    #[test]
    fn configured_executable_must_be_absolute_regular_file() {
        assert!(matches!(
            resolve_git_executable(Some(OsStr::new("relative/git")), None),
            Err(GitCommandError::InvalidExecutable { .. })
        ));

        let directory = TempDir::new().expect("temp directory");
        assert!(matches!(
            resolve_git_executable(Some(directory.path().as_os_str()), None),
            Err(GitCommandError::InvalidExecutable { .. })
        ));
        assert!(matches!(
            resolve_git_executable(Some(directory.path().join("missing").as_os_str()), None),
            Err(GitCommandError::InvalidExecutable { .. })
        ));

        let executable = directory.path().join("git-test");
        fs::write(&executable, b"test executable").expect("write executable");
        let resolved = resolve_git_executable(Some(executable.as_os_str()), None)
            .expect("regular absolute file");
        assert_eq!(
            resolved,
            fs::canonicalize(executable).expect("canonical file")
        );
    }
}
