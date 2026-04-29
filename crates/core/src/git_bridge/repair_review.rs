//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!
//! Read-only repair review data for Git mirror diagnostics.

use serde::{Deserialize, Serialize};

use super::{GitMirrorRecord, GitMirrorRepairAction};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitMirrorRepairReview {
    pub repo_name: String,
    pub manual_only: bool,
    pub authority: String,
    pub records: Vec<GitMirrorRepairReviewRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitMirrorRepairReviewRecord {
    pub deve_commit_id: String,
    pub ledger_seq: u64,
    pub action_code: String,
    pub retryable_after_fix: bool,
    pub subject: String,
    pub next_step: String,
    pub retry_command: Option<String>,
    pub failure_stage: Option<String>,
    pub failure_command: Option<String>,
    pub failure_exit_status: Option<String>,
    pub last_error: Option<String>,
}

pub fn build_repair_review(repo_name: &str, records: &[GitMirrorRecord]) -> GitMirrorRepairReview {
    let retry_command = retry_command(repo_name);
    let records = records
        .iter()
        .filter_map(|record| repair_review_record(record, &retry_command))
        .collect();

    GitMirrorRepairReview {
        repo_name: repo_name.to_string(),
        manual_only: true,
        authority:
            ".notegit/ledger source-control state remains authority; .git is projection mirror only"
                .to_string(),
        records,
    }
}

fn repair_review_record(
    record: &GitMirrorRecord,
    retry_command: &str,
) -> Option<GitMirrorRepairReviewRecord> {
    let action = GitMirrorRepairAction::for_record(record)?;
    Some(GitMirrorRepairReviewRecord {
        deve_commit_id: record.deve_commit_id.clone(),
        ledger_seq: record.ledger_seq,
        action_code: action.code.as_str().to_string(),
        retryable_after_fix: action.retryable_after_fix,
        subject: action.subject.unwrap_or_else(|| "-".to_string()),
        next_step: action.code.next_step().to_string(),
        retry_command: action
            .retryable_after_fix
            .then(|| retry_command.to_string()),
        failure_stage: record.failure_stage.map(|stage| stage.as_str().to_string()),
        failure_command: record.failure_command.clone(),
        failure_exit_status: record.failure_exit_status.clone(),
        last_error: record.last_error.clone(),
    })
}

fn retry_command(repo_name: &str) -> String {
    format!(
        "deve_cli git export --repo {} --retry-out-of-sync",
        shell_quote(repo_name)
    )
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':'))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::build_repair_review;
    use crate::git_bridge::{GitMirrorCommitState, GitMirrorFailureStage, GitMirrorRecord};
    use crate::models::RepoId;

    #[test]
    fn repair_review_contains_only_out_of_sync_actions() {
        let records = vec![
            record("queued", GitMirrorCommitState::Queued),
            record("failed", GitMirrorCommitState::OutOfSync),
        ];

        let review = build_repair_review("repo name", &records);

        assert!(review.manual_only);
        assert_eq!(review.records.len(), 1);
        assert_eq!(review.records[0].deve_commit_id, "failed");
        assert_eq!(review.records[0].action_code, "resolve_projection_scope");
        assert_eq!(review.records[0].subject, "docs/example.md");
        assert_eq!(
            review.records[0].next_step,
            "fix_projection_or_path_subject"
        );
        assert_eq!(
            review.records[0].retry_command.as_deref(),
            Some("deve_cli git export --repo 'repo name' --retry-out-of-sync")
        );
    }

    fn record(deve_commit_id: &str, state: GitMirrorCommitState) -> GitMirrorRecord {
        GitMirrorRecord {
            deve_commit_id: deve_commit_id.to_string(),
            repo_id: RepoId::nil(),
            ledger_seq: 7,
            state,
            git_commit_id: None,
            last_error: Some("projection failed".to_string()),
            failure_stage: Some(GitMirrorFailureStage::ProjectionScope),
            failure_subject: Some("docs/example.md".to_string()),
            failure_command: None,
            failure_exit_status: None,
            queued_at_ms: 1,
            updated_at_ms: 2,
            attempts: 1,
        }
    }
}
