use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, NodeId, Op, PeerId, StructureOp};
use deve_core::sync::SyncManager;
use std::sync::Arc;
use tempfile::TempDir;

mod common;

fn setup_repos() -> (TempDir, Arc<RepoManager>) {
    let dir = TempDir::new().expect("tempdir");
    let ledger = dir.path().join("ledger");
    let mut repo = RepoManager::init(&ledger, 10, Some("main"), Some("urn:main")).expect("main");
    RepoManager::init(&ledger, 10, Some("wiki"), Some("urn:wiki")).expect("wiki");
    repo.set_vault_root(dir.path().join("vault"));
    (dir, Arc::new(repo))
}

fn seed_main_file(repo: &RepoManager) {
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo("main", "notes/live.md", None, "test")
        .expect("create main doc");
    repo.append_generated_op_in_local_repo("main", doc_id, PeerId::new("local"), |seq| {
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "ok".into(),
            },
            1,
            PeerId::new("local"),
            seq,
            None,
            None,
        )
    })
    .expect("append main content");
}

fn inject_broken_structure(repo: &RepoManager) {
    let doc_id = DocId::new();
    common::append_unvalidated_local_op(
        repo,
        "wiki",
        &LedgerEntry::new_structure(
            StructureOp::CreateFile {
                node_id: NodeId::from_doc_id(doc_id),
                doc_id,
                parent_id: Some(NodeId::new()),
                name: "orphan.md".into(),
            },
            1,
            PeerId::new("test"),
            1,
        ),
    );
}

#[test]
fn startup_scan_skips_repo_with_broken_structure_projection() {
    let (dir, repo) = setup_repos();
    seed_main_file(repo.as_ref());
    inject_broken_structure(repo.as_ref());

    let sync = SyncManager::new(repo, dir.path().join("vault"));
    sync.scan().expect("startup scan should skip broken repo");

    assert_eq!(
        std::fs::read_to_string(dir.path().join("vault/main/notes/live.md")).expect("main doc"),
        "ok"
    );
    assert!(!dir.path().join("vault/wiki/orphan.md").exists());
}
