use deve_core::ledger::RepoManager;
use deve_core::models::{LedgerEntry, Op, PeerId};
use deve_core::source_control::{changes, conflict, pending_fs};
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
        PeerId::new("local"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: committed.into(),
                },
                1,
                PeerId::new("local"),
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
        PeerId::new("local"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 5,
                    content: " local".into(),
                },
                2,
                PeerId::new("local"),
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
