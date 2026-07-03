use deve_core::models::DocId;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeStatus, staging};

use super::support::{
    append_confirmed_ledger_edit, ledger_head, new_repo, seed_initial_commit, seed_pending,
    write_workspace_file,
};

#[test]
fn external_stage_rejects_overlap_with_confirmed_ledger_dirty() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    append_confirmed_ledger_edit(&repo, doc_id);
    write_workspace_file(&repo, "notes/a.md", "external");
    seed_pending(&repo, "notes/a.md", ChangeStatus::Modified, "external");

    let err = repo
        .stage_pending("notes/a.md")
        .expect_err("overlapping external change must not stage");

    assert!(
        err.to_string()
            .contains("external change overlaps confirmed ledger changes"),
        "unexpected error: {err}"
    );
    let pending = repo.list_pending_fs().expect("pending retained");
    assert_eq!(pending.len(), 1);
    assert!(
        pending[0].has_conflict,
        "read surface must flag external/confirmed overlap for UI"
    );
    assert!(repo.list_staged().expect("staged remains empty").is_empty());
}

#[test]
fn external_stage_rejects_docless_rename_overlap_with_confirmed_path() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    append_confirmed_ledger_edit(&repo, doc_id);
    write_workspace_file(&repo, "notes/renamed.md", "external");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/renamed.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: None,
                change_type: ChangeStatus::Renamed,
                content_hash: pending_fs::content_hash("external"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed docless rename pending");

    let err = repo
        .stage_pending("notes/renamed.md")
        .expect_err("docless rename overlap must not stage");

    assert!(
        err.to_string()
            .contains("external change overlaps confirmed ledger changes"),
        "unexpected error: {err}"
    );
    let pending = repo.list_pending_fs().expect("pending retained");
    assert_eq!(pending.len(), 1);
    assert!(
        pending[0].has_conflict,
        "docless rename overlap must be surfaced as conflict state"
    );
    assert!(repo.list_staged().expect("staged remains empty").is_empty());
}

#[test]
fn external_stage_fails_closed_on_same_path_different_doc_id_overlap() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    append_confirmed_ledger_edit(&repo, doc_id);
    write_workspace_file(&repo, "notes/a.md", "external");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: Some(DocId::new()),
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("external"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed mismatched doc pending");

    let err = repo
        .stage_pending("notes/a.md")
        .expect_err("same-path different-doc overlap must not stage");

    assert!(
        err.to_string()
            .contains("external change overlaps confirmed ledger changes")
            || err.to_string().contains("Path is not in pending_fs_ops"),
        "unexpected error: {err}"
    );
    let pending = repo.list_pending_fs().expect("pending retained");
    assert_eq!(pending.len(), 1);
    assert!(
        pending[0].has_conflict,
        "same-path doc mismatch must surface as conflict state"
    );
    assert!(repo.list_staged().expect("staged remains empty").is_empty());
}

#[test]
fn external_apply_rejects_staged_overlap_with_confirmed_ledger_dirty() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    write_workspace_file(&repo, "notes/a.md", "external");
    seed_pending(&repo, "notes/a.md", ChangeStatus::Modified, "external");
    repo.stage_pending("notes/a.md")
        .expect("stage external before ledger dirty");
    append_confirmed_ledger_edit(&repo, doc_id);
    let before_apply_head = ledger_head(&repo);

    let err = repo
        .apply_external_changes()
        .expect_err("overlapping staged external change must not apply");

    assert!(
        err.to_string()
            .contains("external change overlaps confirmed ledger changes"),
        "unexpected error: {err}"
    );
    assert_eq!(ledger_head(&repo), before_apply_head);
    let staged = repo.list_staged().expect("staged retained");
    assert_eq!(staged.len(), 1);
    assert!(
        staged[0].has_conflict,
        "staged overlap must be surfaced as conflict state"
    );
    assert_eq!(
        repo.list_confirmed_ledger_changes()
            .expect("confirmed retained")
            .len(),
        1
    );
}

#[test]
fn external_apply_rejects_docless_rename_overlap_with_confirmed_path() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    write_workspace_file(&repo, "notes/renamed.md", "external");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        staging::stage_pending_entry(
            db,
            &PendingFsEntry {
                path: "notes/renamed.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: None,
                change_type: ChangeStatus::Renamed,
                content_hash: pending_fs::content_hash("external"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed docless rename staged");
    append_confirmed_ledger_edit(&repo, doc_id);
    let before_apply_head = ledger_head(&repo);

    let err = repo
        .apply_external_changes()
        .expect_err("docless rename overlap must not apply");

    assert!(
        err.to_string()
            .contains("external change overlaps confirmed ledger changes"),
        "unexpected error: {err}"
    );
    assert_eq!(ledger_head(&repo), before_apply_head);
    let staged = repo.list_staged().expect("staged retained");
    assert_eq!(staged.len(), 1);
    assert!(
        staged[0].has_conflict,
        "docless staged rename overlap must be surfaced as conflict state"
    );
}

#[test]
fn external_apply_rejects_docful_rename_overlap_with_confirmed_path() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_initial_commit(&repo);
    write_workspace_file(&repo, "notes/renamed.md", "external");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        staging::stage_pending_entry(
            db,
            &PendingFsEntry {
                path: "notes/renamed.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: Some(DocId::new()),
                change_type: ChangeStatus::Renamed,
                content_hash: pending_fs::content_hash("external"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed docful mismatched rename staged");
    append_confirmed_ledger_edit(&repo, doc_id);
    let before_apply_head = ledger_head(&repo);

    let err = repo
        .apply_external_changes()
        .expect_err("docful rename overlap must not apply");

    assert!(
        err.to_string()
            .contains("external change overlaps confirmed ledger changes"),
        "unexpected error: {err}"
    );
    assert_eq!(ledger_head(&repo), before_apply_head);
    let staged = repo.list_staged().expect("staged retained");
    assert_eq!(staged.len(), 1);
    assert!(
        staged[0].has_conflict,
        "docful staged rename overlap must be surfaced as conflict state"
    );
}
