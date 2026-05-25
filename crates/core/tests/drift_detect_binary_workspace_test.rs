//! plan_ref:
//!   - 03_storage/projection#projection-contract

use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, Op, PeerId};
use deve_core::sync::SyncManager;
use deve_core::sync::drift_detect::{DriftKind, detect_repo_drift};
use std::sync::Arc;
use tempfile::TempDir;

fn new_repo() -> (TempDir, Arc<RepoManager>) {
    let dir = TempDir::new().expect("create temp dir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    (dir, Arc::new(repo))
}

fn seed_doc(repo: &RepoManager, path: &str, content: &str) -> DocId {
    let (doc_id, _) = repo
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

#[test]
fn binary_workspace_file_is_reported_as_unexpected_not_fatal() {
    let (_dir, repo) = new_repo();
    seed_doc(repo.as_ref(), "notes/a.md", "ledger");
    SyncManager::new_checked(repo.clone())
        .expect("sync manager")
        .materialize_local_repo("default")
        .expect("materialize");

    let rogue = repo
        .local_repo_workspace_path("default", "notes/blob.bin")
        .expect("workspace path");
    std::fs::create_dir_all(rogue.parent().expect("parent")).expect("dirs");
    std::fs::write(&rogue, [0xff, 0xfe, 0x00, 0x01]).expect("write binary");

    let report = detect_repo_drift(repo.as_ref(), "default").expect("detect");
    assert_eq!(report.unexplained.len(), 1);
    assert_eq!(report.unexplained[0].path, "notes/blob.bin");
    assert_eq!(report.unexplained[0].kind, DriftKind::UnexpectedOnDisk);
}
