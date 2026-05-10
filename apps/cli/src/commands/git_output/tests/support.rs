use deve_core::git_bridge::{GitMirrorCommitState, GitMirrorFailureStage, GitMirrorRecord};

pub(crate) fn record(
    id: &str,
    state: GitMirrorCommitState,
    ledger_seq: u64,
    last_error: Option<&str>,
) -> GitMirrorRecord {
    GitMirrorRecord {
        deve_commit_id: id.into(),
        repo_id: uuid::Uuid::nil(),
        ledger_seq,
        state,
        git_commit_id: None,
        last_error: last_error.map(str::to_string),
        failure_stage: last_error.map(GitMirrorFailureStage::classify),
        failure_subject: None,
        failure_command: None,
        failure_exit_status: None,
        queued_at_ms: 1,
        updated_at_ms: 2,
        attempts: 1,
    }
}
