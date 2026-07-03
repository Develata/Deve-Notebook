use deve_core::source_control::{ChangeDomain, ChangeStatus};
use deve_core::sync::SyncManager;
use std::sync::Arc;

#[path = "source_control_external_changes_test/commit_anchor.rs"]
mod commit_anchor;
#[path = "source_control_external_changes_test/discard.rs"]
mod discard;
#[path = "source_control_external_changes_test/overlap.rs"]
mod overlap;
#[path = "source_control_external_changes_test/support.rs"]
mod support;

use support::{
    assert_single_pending_external_change, ledger_head, new_repo, seed_initial_commit,
    seed_pending, workspace_path, write_workspace_file,
};

fn assert_apply_external_changes_writes_ledger_without_commit_anchor() {
    let (_dir, repo) = new_repo();
    let before_head = ledger_head(&repo);
    write_workspace_file(&repo, "notes/external.md", "external");
    seed_pending(&repo, "notes/external.md", ChangeStatus::Added, "external");
    repo.stage_pending("notes/external.md")
        .expect("stage external add");

    let confirmed = repo
        .apply_external_changes()
        .expect("apply external changes");

    assert!(ledger_head(&repo) > before_head);
    assert!(repo.list_staged().expect("staged after apply").is_empty());
    assert!(
        repo.list_pending_fs()
            .expect("pending after apply")
            .is_empty()
    );
    assert!(
        repo.list_commits(10)
            .expect("commits after external apply")
            .is_empty(),
        "Apply to Ledger must not create a Source Control commit anchor"
    );
    assert!(confirmed.iter().any(|entry| {
        entry.path == "notes/external.md"
            && entry.domain == ChangeDomain::ConfirmedLedger
            && entry.status == ChangeStatus::Added
    }));
}

#[test]
fn external_changes_apply_writes_ledger_without_commit_anchor() {
    assert_apply_external_changes_writes_ledger_without_commit_anchor();
}

#[test]
fn apply_external_changes_to_ledger() {
    assert_apply_external_changes_writes_ledger_without_commit_anchor();
}

#[test]
fn external_file_changes_enter_external_changes_not_ledger() {
    let (_dir, repo) = new_repo();
    let repo = Arc::new(repo);
    let sync = SyncManager::new_checked(Arc::clone(&repo)).expect("sync manager");
    sync.scan().expect("materialize clean projection workspace");
    let before_head = ledger_head(repo.as_ref());

    write_workspace_file(repo.as_ref(), "notes/external.md", "external");
    sync.scan().expect("scan projection workspace");

    assert_single_pending_external_change(
        repo.as_ref(),
        "notes/external.md",
        None,
        ChangeStatus::Added,
        before_head,
    );
}

#[test]
fn external_scan_modified_enters_external_changes_not_ledger() {
    let (_dir, repo) = new_repo();
    let repo = Arc::new(repo);
    let sync = SyncManager::new_checked(Arc::clone(&repo)).expect("sync manager");
    sync.scan().expect("materialize clean projection workspace");
    let doc_id = seed_initial_commit(repo.as_ref());
    let before_head = ledger_head(repo.as_ref());

    write_workspace_file(repo.as_ref(), "notes/a.md", "hello external");
    sync.scan().expect("scan modified projection file");

    assert_single_pending_external_change(
        repo.as_ref(),
        "notes/a.md",
        Some(doc_id),
        ChangeStatus::Modified,
        before_head,
    );
}

#[test]
fn external_scan_deleted_enters_external_changes_not_ledger() {
    let (_dir, repo) = new_repo();
    let repo = Arc::new(repo);
    let sync = SyncManager::new_checked(Arc::clone(&repo)).expect("sync manager");
    sync.scan().expect("materialize clean projection workspace");
    let doc_id = seed_initial_commit(repo.as_ref());
    let before_head = ledger_head(repo.as_ref());

    std::fs::remove_file(workspace_path(repo.as_ref(), "notes/a.md"))
        .expect("delete projection file");
    sync.scan().expect("scan deleted projection file");

    assert_single_pending_external_change(
        repo.as_ref(),
        "notes/a.md",
        Some(doc_id),
        ChangeStatus::Deleted,
        before_head,
    );
}

#[test]
fn external_scan_renamed_enters_external_changes_not_ledger() {
    let (_dir, repo) = new_repo();
    let repo = Arc::new(repo);
    let sync = SyncManager::new_checked(Arc::clone(&repo)).expect("sync manager");
    sync.scan().expect("materialize clean projection workspace");
    let doc_id = seed_initial_commit(repo.as_ref());
    sync.scan().expect("bind committed projection identity");
    assert!(
        repo.list_pending_fs()
            .expect("pending before external rename")
            .is_empty()
    );
    let before_head = ledger_head(repo.as_ref());

    std::fs::rename(
        workspace_path(repo.as_ref(), "notes/a.md"),
        workspace_path(repo.as_ref(), "notes/renamed.md"),
    )
    .expect("rename projection file");
    sync.scan().expect("scan renamed projection file");

    let pending = repo.list_pending_fs().expect("pending external rename");
    assert_eq!(pending.len(), 2);
    assert!(pending.iter().any(|entry| {
        entry.path == "notes/a.md"
            && entry.doc_id == Some(doc_id)
            && entry.status == ChangeStatus::Deleted
            && entry.renamed_from.is_none()
            && entry.domain == ChangeDomain::WorkingDirectory
    }));
    assert!(pending.iter().any(|entry| {
        entry.path == "notes/renamed.md"
            && entry.doc_id == Some(doc_id)
            && entry.status == ChangeStatus::Added
            && entry.renamed_from.as_deref() == Some("notes/a.md")
            && entry.domain == ChangeDomain::WorkingDirectory
    }));
    assert_eq!(ledger_head(repo.as_ref()), before_head);
    assert!(
        repo.list_confirmed_ledger_changes()
            .expect("confirmed remains clean")
            .is_empty()
    );
    assert!(repo.list_staged().expect("staged remains clean").is_empty());
}

#[test]
fn external_stage_unstage_only_moves_external_staging() {
    let (_dir, repo) = new_repo();
    let before_head = ledger_head(&repo);

    write_workspace_file(&repo, "notes/external.md", "external");
    seed_pending(&repo, "notes/external.md", ChangeStatus::Added, "external");
    repo.stage_pending("notes/external.md")
        .expect("stage external");

    assert!(
        repo.list_pending_fs()
            .expect("pending after stage")
            .is_empty()
    );
    let staged = repo.list_staged().expect("staged external");
    assert_eq!(staged.len(), 1);
    assert_eq!(staged[0].path, "notes/external.md");
    assert_eq!(staged[0].domain, ChangeDomain::Staged);
    assert_eq!(ledger_head(&repo), before_head);

    repo.unstage_file("notes/external.md")
        .expect("unstage external");

    assert!(repo.list_staged().expect("staged after unstage").is_empty());
    let pending = repo.list_pending_fs().expect("pending after unstage");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "notes/external.md");
    assert_eq!(pending[0].domain, ChangeDomain::WorkingDirectory);
    assert_eq!(ledger_head(&repo), before_head);
}

#[test]
fn source_control_confirmed_ledger_changes_visible_after_apply() {
    let (_dir, repo) = new_repo();

    write_workspace_file(&repo, "notes/external.md", "external");
    seed_pending(&repo, "notes/external.md", ChangeStatus::Added, "external");
    repo.stage_pending("notes/external.md")
        .expect("stage external add");
    repo.apply_external_changes()
        .expect("apply external changes");

    let confirmed = repo
        .list_confirmed_ledger_changes()
        .expect("confirmed after apply");
    assert_eq!(confirmed.len(), 1);
    assert_eq!(confirmed[0].path, "notes/external.md");
    assert_eq!(confirmed[0].status, ChangeStatus::Added);
    assert_eq!(confirmed[0].domain, ChangeDomain::ConfirmedLedger);
}
