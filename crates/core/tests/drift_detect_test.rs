//! plan_ref:
//!   - 04_storage.md §Non-Negotiable Invariants

use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, Op, PeerId};
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::sync::SyncManager;
use deve_core::sync::drift_detect::{DriftKind, detect_repo_drift};
use std::sync::Arc;
use tempfile::TempDir;

fn new_repo() -> (TempDir, Arc<RepoManager>) {
    let dir = TempDir::new().expect("create temp dir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init");
    repo.set_vault_root(dir.path().join("vault"));
    (dir, Arc::new(repo))
}

fn seed_doc(repo: &RepoManager, path: &str, content: &str) -> DocId {
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo(repo.local_repo_name(), path, None, "test")
        .expect("structure");
    repo.append_generated_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        PeerId::new("local"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: content.into(),
                },
                1,
                PeerId::new("local"),
                seq,
                None,
                None,
            )
        },
    )
    .expect("append op");
    doc_id
}

fn materialize(repo: &Arc<RepoManager>, dir: &TempDir) {
    let sync = SyncManager::new(repo.clone(), dir.path().join("vault"));
    sync.materialize_local_repo("default").expect("materialize");
}

#[test]
fn clean_workspace_reports_no_drift() {
    let (dir, repo) = new_repo();
    seed_doc(repo.as_ref(), "notes/a.md", "ledger");
    materialize(&repo, &dir);

    let report = detect_repo_drift(repo.as_ref(), "default").expect("detect");
    assert_eq!(report.explained_count, 0);
    assert!(!report.is_fault());
    assert!(report.unexplained.is_empty());
}

#[test]
fn pending_modify_is_explained() {
    let (dir, repo) = new_repo();
    let doc_id = seed_doc(repo.as_ref(), "notes/a.md", "ledger");
    materialize(&repo, &dir);

    let file_path = dir.path().join("vault/default/notes/a.md");
    std::fs::write(&file_path, "workspace").expect("write");
    let hash = pending_fs::content_hash("workspace");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Modified,
                content_hash: hash.clone(),
                detected_at: 0,
                has_conflict: false,
            },
        )
    })
    .expect("pending");

    let report = detect_repo_drift(repo.as_ref(), "default").expect("detect");
    assert!(report.unexplained.is_empty());
    assert_eq!(report.explained_count, 1);
}

#[test]
fn unexplained_orphan_file_is_fault() {
    let (dir, repo) = new_repo();
    seed_doc(repo.as_ref(), "notes/a.md", "ledger");
    materialize(&repo, &dir);

    let orphan_path = dir.path().join("vault/default/notes/orphan.md");
    std::fs::create_dir_all(orphan_path.parent().unwrap()).expect("dirs");
    std::fs::write(&orphan_path, "orphan").expect("write");

    let report = detect_repo_drift(repo.as_ref(), "default").expect("detect");
    assert_eq!(report.unexplained.len(), 1);
    assert_eq!(report.unexplained[0].kind, DriftKind::UnexpectedOnDisk);
    assert!(report.is_fault());
}

#[test]
fn unexplained_missing_file_is_fault() {
    let (dir, repo) = new_repo();
    seed_doc(repo.as_ref(), "notes/a.md", "ledger");
    materialize(&repo, &dir);

    let tracked_path = dir.path().join("vault/default/notes/a.md");
    std::fs::remove_file(tracked_path).expect("remove");

    let report = detect_repo_drift(repo.as_ref(), "default").expect("detect");
    assert_eq!(report.unexplained.len(), 1);
    assert_eq!(report.unexplained[0].kind, DriftKind::MissingOnDisk);
    assert!(report.is_fault());
}
