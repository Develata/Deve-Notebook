//! plan_ref:
//!   - 03_storage#git-ecosystem-coexistence
//!   - 05_diff_logic#git-mirror-lifecycle
//!
//! Compatibility parser for Git mirror failure metadata.

use super::store::GitMirrorFailureStage;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct GitMirrorFailureMetadata {
    pub subject: Option<String>,
    pub command: Option<String>,
    pub exit_status: Option<String>,
}

impl GitMirrorFailureMetadata {
    pub(super) fn from_error(stage: GitMirrorFailureStage, error: &str) -> Self {
        Self {
            subject: failure_subject(stage, error),
            command: failure_command(error),
            exit_status: failure_exit_status(error),
        }
    }
}

fn failure_subject(stage: GitMirrorFailureStage, error: &str) -> Option<String> {
    let subject = match stage {
        GitMirrorFailureStage::NotegitProtection => ".notegit".to_string(),
        GitMirrorFailureStage::ProjectionScope => extract_after_any(
            error,
            &[
                "path(s) outside queued Deve commit: ",
                "path(s) outside queued Deve commits: ",
                "path(s) outside current Deve projection snapshot: ",
                "unsafe projection path: ",
            ],
        )?
        .to_string(),
        GitMirrorFailureStage::GitWorktree => {
            extract_after_any(error, &["dirty Git worktree path(s): "])?.to_string()
        }
        GitMirrorFailureStage::GitHistoryMapping => extract_after_any(
            error,
            &[
                "parent ",
                "mirrored parent ",
                "requires empty Git history, but HEAD is ",
            ],
        )?
        .to_string(),
        _ => return None,
    };
    let subject = subject.trim().trim_end_matches('.').to_string();
    if subject.is_empty() {
        None
    } else {
        Some(subject)
    }
}

fn failure_command(error: &str) -> Option<String> {
    if let Some(command) = extract_between(error, "git ", " failed") {
        return Some(command);
    }
    extract_after_any(
        error,
        &[
            "failed to run git ",
            "failed to open stdin for git ",
            "failed to write stdin for git ",
            "failed to wait for git ",
        ],
    )
    .map(|value| value.split(':').next().unwrap_or(value).trim().to_string())
    .filter(|value| !value.is_empty())
}

fn failure_exit_status(error: &str) -> Option<String> {
    if let Some(status) = extract_between(error, "(status ", ")") {
        return Some(status);
    }
    extract_after_any(error, &["failed with status "]).map(|value| {
        value
            .split(':')
            .take(2)
            .collect::<Vec<_>>()
            .join(":")
            .trim()
            .to_string()
    })
}

fn extract_after_any<'a>(value: &'a str, needles: &[&str]) -> Option<&'a str> {
    needles
        .iter()
        .find_map(|needle| value.split_once(needle).map(|(_, tail)| tail))
}

fn extract_between(value: &str, start: &str, end: &str) -> Option<String> {
    let (_, tail) = value.split_once(start)?;
    let (head, _) = tail.split_once(end)?;
    let head = head.trim();
    if head.is_empty() {
        None
    } else {
        Some(head.to_string())
    }
}
