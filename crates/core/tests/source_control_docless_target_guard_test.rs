use deve_core::ledger::RepoManager;
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeStatus, SourceControlApi, staging};
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos(dir.path().join("notes"));
    (dir, repo)
}

#[test]
fn stage_path_only_target_fails_closed_for_docless_reused_old_path() {
    let (_dir, repo) = new_repo();
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/old.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("old exact"),
                detected_at: 1,
                has_conflict: false,
            },
        )?;
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/new.md".into(),
                renamed_from: Some("notes/old.md".into()),
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("rename successor"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })
    .expect("seed pending");

    let err = <RepoManager as SourceControlApi>::stage_pending_in_repo(
        &repo,
        &RepoSelector::default(),
        &ScPathTarget::from_path("notes/old.md"),
    )
    .expect_err("ambiguous docless stage must fail closed");
    assert!(
        err.to_string()
            .contains("Ambiguous pending_fs target: notes/old.md")
    );
}

#[test]
fn unstage_path_only_target_fails_closed_for_docless_reused_old_path() {
    let (_dir, repo) = new_repo();
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        staging::stage_pending_entry(
            db,
            &PendingFsEntry {
                path: "notes/old.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("old exact"),
                detected_at: 1,
                has_conflict: false,
            },
        )?;
        staging::stage_pending_entry(
            db,
            &PendingFsEntry {
                path: "notes/new.md".into(),
                renamed_from: Some("notes/old.md".into()),
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("rename successor"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })
    .expect("seed staged");

    let err = <RepoManager as SourceControlApi>::unstage_file_in_repo(
        &repo,
        &RepoSelector::default(),
        &ScPathTarget::from_path("notes/old.md"),
    )
    .expect_err("ambiguous docless unstage must fail closed");
    assert!(
        err.to_string()
            .contains("Ambiguous staged target: notes/old.md")
    );
}
