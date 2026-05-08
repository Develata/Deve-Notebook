//! plan_ref:
//!   - 07_diff_logic#git-mirror-lifecycle

pub(super) type GitBridgeResult<T> = std::result::Result<T, GitBridgeError>;
pub(super) type GitCommandResult<T> = std::result::Result<T, GitCommandError>;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(super) enum GitBridgeError {
    #[error("Git push mirror refuses invalid {label}: {value:?}")]
    InvalidPushName { label: &'static str, value: String },
    #[error("Git push mirror requires a named branch; detached HEAD needs --branch")]
    DetachedHead,
    #[error(transparent)]
    GitCommand(#[from] GitCommandError),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(super) enum GitCommandError {
    #[error("failed to run git {args}: {message}")]
    Spawn { args: String, message: String },
    #[error("failed to open stdin for git {args}")]
    MissingStdin { args: String },
    #[error("failed to write stdin for git {args}: {message}")]
    StdinWrite { args: String, message: String },
    #[error("failed to wait for git {args}: {message}")]
    Wait { args: String, message: String },
    #[error("git {args} returned non-UTF-8 field: {message}")]
    NonUtf8Field { args: String, message: String },
    #[error("git {args} failed with status {status}")]
    Status { args: String, status: String },
    #[error("git {args} failed (status {status}): {detail}")]
    StatusDetail {
        args: String,
        status: String,
        detail: String,
    },
}

impl GitCommandError {
    pub(super) fn status(args: &[&str], output: &std::process::Output) -> Self {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        let args = args.join(" ");
        let status = output.status.to_string();
        if detail.is_empty() {
            return Self::Status { args, status };
        }
        Self::StatusDetail {
            args,
            status,
            detail,
        }
    }
}

impl From<GitCommandError> for String {
    fn from(err: GitCommandError) -> Self {
        err.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::GitCommandError;

    #[test]
    fn git_command_error_preserves_legacy_status_text() {
        assert_eq!(
            GitCommandError::Status {
                args: "status".into(),
                status: "exit status: 1".into(),
            }
            .to_string(),
            "git status failed with status exit status: 1"
        );
        assert_eq!(
            GitCommandError::StatusDetail {
                args: "push origin main".into(),
                status: "exit status: 128".into(),
                detail: "fatal: rejected".into(),
            }
            .to_string(),
            "git push origin main failed (status exit status: 128): fatal: rejected"
        );
    }

    #[test]
    fn git_command_error_converts_to_string_for_legacy_callers() {
        let message: String = GitCommandError::Spawn {
            args: "rev-parse HEAD".into(),
            message: "No such file or directory".into(),
        }
        .into();

        assert_eq!(
            message,
            "failed to run git rev-parse HEAD: No such file or directory"
        );
    }
}
