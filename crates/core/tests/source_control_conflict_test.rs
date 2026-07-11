use deve_core::ledger::RepoManager;
use deve_core::models::{LedgerEntry, Op};
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::pending_fs::PendingFsEntry;
use deve_core::source_control::{ChangeStatus, changes, conflict, pending_fs, staging};
use tempfile::TempDir;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = TempDir::new().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    (dir, repo)
}

fn seed_doc(repo: &RepoManager, path: &str, committed: &str) -> deve_core::models::DocId {
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo(repo.local_repo_name(), path, None, "test")
        .expect("create file structure");
    repo.append_generated_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        repo.local_peer_id().clone(),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: committed.into(),
                },
                1,
                repo.local_peer_id().clone(),
                seq,
                None,
                None,
            )
        },
    )
    .expect("append committed content");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        changes::save_snapshot(db, doc_id, committed)
    })
    .expect("save snapshot");
    doc_id
}

#[test]
fn check_conflict_detects_ledger_divergence_against_snapshot() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_doc(&repo, "notes/a.md", "hello");
    repo.append_generated_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        repo.local_peer_id().clone(),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 5,
                    content: " local".into(),
                },
                2,
                repo.local_peer_id().clone(),
                seq,
                None,
                None,
            )
        },
    )
    .expect("append local-only op");

    let pending_hash = pending_fs::content_hash("hello fs");
    let has_conflict = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            conflict::check_conflict(db, doc_id, &pending_hash)
        })
        .expect("check conflict");
    assert!(has_conflict);
}

#[test]
fn check_conflict_is_false_without_ledger_divergence() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_doc(&repo, "notes/a.md", "hello");
    let pending_hash = pending_fs::content_hash("hello fs");
    let has_conflict = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            conflict::check_conflict(db, doc_id, &pending_hash)
        })
        .expect("check conflict");
    assert!(!has_conflict);
}

#[test]
fn stage_pending_rejects_unresolved_conflict() {
    let (_dir, repo) = new_repo();
    let doc_id = seed_doc(&repo, "notes/a.md", "hello");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("hello fs"),
                detected_at: 1,
                has_conflict: true,
            },
        )
    })
    .expect("seed conflict pending");

    let err = repo
        .stage_pending_target_in_local_repo(
            repo.local_repo_name(),
            &ScPathTarget {
                path: "notes/a.md".into(),
                doc_id: Some(doc_id),
                domain: None,
            },
        )
        .expect_err("unresolved conflict must not stage");

    assert!(
        err.to_string()
            .contains("unresolved source control conflict"),
        "unexpected error: {}",
        err
    );
    let pending = repo
        .run_on_local_repo(repo.local_repo_name(), pending_fs::list_all)
        .expect("pending retained");
    let staged = repo
        .run_on_local_repo(repo.local_repo_name(), staging::list_staged_entries)
        .expect("staged empty");
    assert_eq!(pending.len(), 1);
    assert!(pending[0].has_conflict);
    assert!(staged.is_empty());
}
