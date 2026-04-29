use super::status_lines_at;
use deve_core::git_bridge::{
    GitMetadataKind, GitMirrorCommitState, GitMirrorFailureStage, GitMirrorRecord, GitMirrorState,
    GitMirrorStatus, GitMirrorSummary,
};

fn out_of_sync_record(error: &str) -> GitMirrorRecord {
    GitMirrorRecord {
        deve_commit_id: "deve-1".into(),
        repo_id: uuid::Uuid::nil(),
        ledger_seq: 7,
        state: GitMirrorCommitState::OutOfSync,
        git_commit_id: None,
        last_error: Some(error.to_string()),
        failure_stage: Some(GitMirrorFailureStage::classify(error)),
        failure_subject: Some("docs/example.md".to_string()),
        failure_command: None,
        failure_exit_status: None,
        queued_at_ms: 1,
        updated_at_ms: 2,
        attempts: 1,
    }
}

fn ready_status() -> GitMirrorStatus {
    GitMirrorStatus {
        repo_root: std::path::PathBuf::from("vault/default"),
        notegit_present: true,
        git_metadata_kind: GitMetadataKind::Directory,
        gitignore_protects_notegit: true,
        state: GitMirrorState::Ready,
        reason: None,
    }
}

#[test]
fn status_lines_include_cli_only_repair_action() {
    let lines = status_lines_at(
        "default",
        &ready_status(),
        &GitMirrorSummary {
            queued: 0,
            committed: 0,
            out_of_sync: 1,
        },
        &[out_of_sync_record(
            "Git mirror refuses unsafe projection path: docs/example.md",
        )],
        11,
    );

    assert!(lines.iter().any(|line| {
        line.contains(
            "repair_action[1]: code=resolve_projection_scope retryable_after_fix=yes subject=docs/example.md",
        )
    }));
}
