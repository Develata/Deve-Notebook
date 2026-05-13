//! plan_ref:
//!   - 07_diff_logic#git-mirror-lifecycle
//!   - 12_commands#cli-commands
//!

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(in crate::git_bridge) enum GitBridgeError {
    #[error("Git push mirror refuses invalid {label}: {value:?}")]
    InvalidPushName { label: &'static str, value: String },
    #[error("Git push mirror requires a named branch; detached HEAD needs --branch")]
    DetachedHead,
    #[error(transparent)]
    GitCommand(#[from] GitCommandError),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(in crate::git_bridge) enum GitCommandError {
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
    pub(in crate::git_bridge) fn status(args: &[&str], output: &std::process::Output) -> Self {
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
