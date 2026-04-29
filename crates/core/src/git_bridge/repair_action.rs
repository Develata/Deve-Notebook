//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!
//! CLI-only repair action schema for Git mirror out-of-sync diagnostics.

use super::store::{GitMirrorCommitState, GitMirrorFailureStage, GitMirrorRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitMirrorRepairActionCode {
    PrepareMirror,
    CleanDeveSourceControl,
    ProtectNotegit,
    ResolveProjectionScope,
    RepairHistoryMapping,
    CleanGitWorktree,
    InspectGitCommand,
    InspectMirrorExecutor,
}

impl GitMirrorRepairActionCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PrepareMirror => "prepare_mirror",
            Self::CleanDeveSourceControl => "clean_deve_source_control",
            Self::ProtectNotegit => "protect_notegit",
            Self::ResolveProjectionScope => "resolve_projection_scope",
            Self::RepairHistoryMapping => "repair_history_mapping",
            Self::CleanGitWorktree => "clean_git_worktree",
            Self::InspectGitCommand => "inspect_git_command",
            Self::InspectMirrorExecutor => "inspect_mirror_executor",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitMirrorRepairAction {
    pub code: GitMirrorRepairActionCode,
    pub retryable_after_fix: bool,
    pub subject: Option<String>,
}

impl GitMirrorRepairAction {
    pub fn for_record(record: &GitMirrorRecord) -> Option<Self> {
        if record.state != GitMirrorCommitState::OutOfSync {
            return None;
        }
        let stage = record.failure_stage.or_else(|| {
            record
                .last_error
                .as_deref()
                .map(GitMirrorFailureStage::classify)
        })?;
        Some(Self {
            code: action_code(stage),
            retryable_after_fix: retryable_after_fix(stage),
            subject: record.failure_subject.clone(),
        })
    }
}

fn action_code(stage: GitMirrorFailureStage) -> GitMirrorRepairActionCode {
    match stage {
        GitMirrorFailureStage::MirrorNotReady => GitMirrorRepairActionCode::PrepareMirror,
        GitMirrorFailureStage::DeveSourceControl => {
            GitMirrorRepairActionCode::CleanDeveSourceControl
        }
        GitMirrorFailureStage::NotegitProtection => GitMirrorRepairActionCode::ProtectNotegit,
        GitMirrorFailureStage::ProjectionScope => GitMirrorRepairActionCode::ResolveProjectionScope,
        GitMirrorFailureStage::GitHistoryMapping => GitMirrorRepairActionCode::RepairHistoryMapping,
        GitMirrorFailureStage::GitWorktree => GitMirrorRepairActionCode::CleanGitWorktree,
        GitMirrorFailureStage::GitCommand => GitMirrorRepairActionCode::InspectGitCommand,
        GitMirrorFailureStage::MirrorExecutor => GitMirrorRepairActionCode::InspectMirrorExecutor,
    }
}

fn retryable_after_fix(stage: GitMirrorFailureStage) -> bool {
    !matches!(stage, GitMirrorFailureStage::MirrorExecutor)
}

#[cfg(test)]
mod tests {
    use crate::models::RepoId;

    use super::*;

    fn out_of_sync_record(stage: GitMirrorFailureStage) -> GitMirrorRecord {
        GitMirrorRecord {
            deve_commit_id: "deve-1".to_string(),
            repo_id: RepoId::nil(),
            ledger_seq: 7,
            state: GitMirrorCommitState::OutOfSync,
            git_commit_id: None,
            last_error: Some("failure".to_string()),
            failure_stage: Some(stage),
            failure_subject: Some("subject.md".to_string()),
            failure_command: None,
            failure_exit_status: None,
            queued_at_ms: 1,
            updated_at_ms: 2,
            attempts: 1,
        }
    }

    #[test]
    fn repair_action_maps_structured_failure_stage() {
        let action = GitMirrorRepairAction::for_record(&out_of_sync_record(
            GitMirrorFailureStage::GitCommand,
        ))
        .expect("repair action");

        assert_eq!(action.code, GitMirrorRepairActionCode::InspectGitCommand);
        assert!(action.retryable_after_fix);
        assert_eq!(action.subject.as_deref(), Some("subject.md"));
    }

    #[test]
    fn repair_action_ignores_non_out_of_sync_records() {
        let mut record = out_of_sync_record(GitMirrorFailureStage::GitCommand);
        record.state = GitMirrorCommitState::Queued;

        assert_eq!(GitMirrorRepairAction::for_record(&record), None);
    }
}
