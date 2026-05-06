//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!
//! Git mirror persistent record schema and failure-state taxonomy.

use crate::models::RepoId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitMirrorCommitState {
    Queued,
    Committed,
    OutOfSync,
}

impl GitMirrorCommitState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Committed => "committed",
            Self::OutOfSync => "out_of_sync",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitMirrorFailureStage {
    MirrorNotReady,
    DeveSourceControl,
    NotegitProtection,
    ProjectionScope,
    GitHistoryMapping,
    GitWorktree,
    GitCommand,
    MirrorExecutor,
}

impl GitMirrorFailureStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MirrorNotReady => "mirror_not_ready",
            Self::DeveSourceControl => "deve_source_control",
            Self::NotegitProtection => "notegit_protection",
            Self::ProjectionScope => "projection_scope",
            Self::GitHistoryMapping => "git_history_mapping",
            Self::GitWorktree => "git_worktree",
            Self::GitCommand => "git_command",
            Self::MirrorExecutor => "mirror_executor",
        }
    }

    pub fn classify(error: &str) -> Self {
        let normalized = error.to_ascii_lowercase();
        if normalized.contains("not ready:")
            || normalized.contains("protectionmissing")
            || normalized.contains("does not ignore .notegit")
        {
            return Self::MirrorNotReady;
        }
        if normalized.contains("pending source-control")
            || normalized.contains("staged source-control")
            || normalized.contains("pending_fs")
            || normalized.contains("staging")
        {
            return Self::DeveSourceControl;
        }
        if normalized.contains("outside queued deve commit")
            || normalized.contains("outside current deve projection snapshot")
            || normalized.contains("projection diff")
        {
            return Self::ProjectionScope;
        }
        if normalized.contains(".notegit") || normalized.contains("tracked by git") {
            return Self::NotegitProtection;
        }
        if normalized.contains("unsafe projection path") {
            return Self::ProjectionScope;
        }
        if normalized.contains("parent")
            || normalized.contains("git head does not match")
            || normalized.contains("requires empty git history")
            || normalized.contains("not mirrored")
        {
            return Self::GitHistoryMapping;
        }
        if normalized.contains("worktree") || normalized.contains("rev-parse") {
            return Self::GitWorktree;
        }
        if normalized.starts_with("git mirror ") {
            return Self::MirrorExecutor;
        }
        if normalized.starts_with("git ")
            || normalized.starts_with("failed to run git ")
            || normalized.contains(" for git ")
            || normalized.contains(" stdin for git ")
        {
            return Self::GitCommand;
        }
        Self::MirrorExecutor
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitMirrorRecord {
    pub deve_commit_id: String,
    pub repo_id: RepoId,
    pub ledger_seq: u64,
    pub state: GitMirrorCommitState,
    #[serde(default)]
    pub git_commit_id: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub failure_stage: Option<GitMirrorFailureStage>,
    #[serde(default)]
    pub failure_subject: Option<String>,
    #[serde(default)]
    pub failure_command: Option<String>,
    #[serde(default)]
    pub failure_exit_status: Option<String>,
    pub queued_at_ms: i64,
    pub updated_at_ms: i64,
    #[serde(default)]
    pub attempts: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitMirrorSummary {
    pub queued: usize,
    pub committed: usize,
    pub out_of_sync: usize,
}
