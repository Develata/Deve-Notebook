use super::{
    GitMirrorCommitState, GitMirrorFailureStage, GitMirrorRecord, get_record, init_table,
    mark_committed, mark_out_of_sync, queue_deve_commit, summarize_records,
};
use crate::source_control::CommitInfo;

fn commit(id: &str, ledger_seq: u64) -> CommitInfo {
    CommitInfo {
        id: id.to_string(),
        parent_id: None,
        message: "commit".to_string(),
        timestamp: 1,
        doc_count: 1,
        ledger_seq,
    }
}

#[test]
fn queue_deve_commit_is_idempotent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = redb::Database::create(dir.path().join("mirror.redb")).expect("db");
    init_table(&db).expect("init");
    let repo_id = uuid::Uuid::new_v4();

    let first = queue_deve_commit(&db, repo_id, &commit("c1", 7)).expect("queue");
    let second = queue_deve_commit(&db, repo_id, &commit("c1", 7)).expect("queue again");

    assert_eq!(first, second);
    assert_eq!(first.state, GitMirrorCommitState::Queued);
    assert_eq!(first.repo_id, repo_id);
    assert_eq!(first.ledger_seq, 7);
}

#[test]
fn mark_committed_and_out_of_sync_update_summary() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = redb::Database::create(dir.path().join("mirror.redb")).expect("db");
    init_table(&db).expect("init");
    let repo_id = uuid::Uuid::new_v4();
    queue_deve_commit(&db, repo_id, &commit("c1", 1)).expect("queue c1");
    queue_deve_commit(&db, repo_id, &commit("c2", 2)).expect("queue c2");

    mark_committed(&db, "c1", "abc123").expect("mark committed");
    let failed = mark_out_of_sync(
        &db,
        "c2",
        "git commit failed (status exit status: 128): missing user.name",
    )
    .expect("mark failed");

    let summary = summarize_records(&db).expect("summary");
    assert_eq!(summary.queued, 0);
    assert_eq!(summary.committed, 1);
    assert_eq!(summary.out_of_sync, 1);
    assert_eq!(failed.state, GitMirrorCommitState::OutOfSync);
    assert_eq!(
        failed.last_error.as_deref(),
        Some("git commit failed (status exit status: 128): missing user.name")
    );
    assert_eq!(
        failed.failure_stage,
        Some(GitMirrorFailureStage::GitCommand)
    );
    assert_eq!(failed.failure_command.as_deref(), Some("commit"));
    assert_eq!(
        failed.failure_exit_status.as_deref(),
        Some("exit status: 128")
    );
    assert_eq!(
        get_record(&db, "c1")
            .expect("get")
            .and_then(|record| record.git_commit_id),
        Some("abc123".to_string())
    );
}

#[test]
fn legacy_record_without_failure_stage_still_decodes() {
    let raw = serde_json::json!({
        "deve_commit_id": "legacy",
        "repo_id": uuid::Uuid::nil(),
        "ledger_seq": 1,
        "state": "OutOfSync",
        "git_commit_id": null,
        "last_error": "old error",
        "queued_at_ms": 1,
        "updated_at_ms": 2,
        "attempts": 1
    })
    .to_string();

    let record: GitMirrorRecord = serde_json::from_str(&raw).expect("decode legacy");

    assert_eq!(record.deve_commit_id, "legacy");
    assert_eq!(record.failure_stage, None);
    assert_eq!(record.failure_subject, None);
    assert_eq!(record.failure_command, None);
    assert_eq!(record.failure_exit_status, None);
}

#[test]
fn out_of_sync_metadata_extracts_offending_subjects() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = redb::Database::create(dir.path().join("mirror.redb")).expect("db");
    init_table(&db).expect("init");
    let repo_id = uuid::Uuid::new_v4();
    queue_deve_commit(&db, repo_id, &commit("c1", 1)).expect("queue c1");

    let failed = mark_out_of_sync(
        &db,
        "c1",
        "Git mirror refuses to include path(s) outside queued Deve commit: outside.md, .notegit/state",
    )
    .expect("mark failed");

    assert_eq!(
        failed.failure_stage,
        Some(GitMirrorFailureStage::ProjectionScope)
    );
    assert_eq!(
        failed.failure_subject.as_deref(),
        Some("outside.md, .notegit/state")
    );
    assert_eq!(failed.failure_command, None);
    assert_eq!(failed.failure_exit_status, None);
}

#[test]
fn failure_stage_classification_covers_known_locations() {
    assert_eq!(
        GitMirrorFailureStage::classify(
            "Git mirror refuses to run with 1 pending source-control change(s)"
        ),
        GitMirrorFailureStage::DeveSourceControl
    );
    assert_eq!(
        GitMirrorFailureStage::classify("Git mirror refuses unsafe projection path: .notegit"),
        GitMirrorFailureStage::NotegitProtection
    );
    assert_eq!(
        GitMirrorFailureStage::classify(
            "Git mirror refuses to include path(s) outside queued Deve commit"
        ),
        GitMirrorFailureStage::ProjectionScope
    );
    assert_eq!(
        GitMirrorFailureStage::classify(
            "Git mirror snapshot bootstrap requires empty Git history, but HEAD is abc"
        ),
        GitMirrorFailureStage::GitHistoryMapping
    );
    assert_eq!(
        GitMirrorFailureStage::classify("repo-local .gitignore does not ignore .notegit/"),
        GitMirrorFailureStage::MirrorNotReady
    );
}
