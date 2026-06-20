use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, NodeId, Op, PeerId, StructureOp};
use deve_core::sync::SyncManager;
use std::sync::Arc;
use tempfile::TempDir;

mod common;

fn setup_repos() -> (TempDir, Arc<RepoManager>) {
    let dir = TempDir::new().expect("tempdir");
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 10, Some("main"), Some("urn:main")).expect("main");
    common::create_initialized_local_repo_with_depth(&ledger, 10, "wiki", "urn:wiki");
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)
        .expect("projection base");
    // Mirror production init order: establish workspace identity markers before any
    // projection content exists, so the scan's identity gate is exercised against
    // legitimately-owned workspaces rather than tripping on missing markers.
    repo.ensure_local_repo_workspace_identity("main")
        .expect("main workspace identity");
    repo.ensure_local_repo_workspace_identity("wiki")
        .expect("wiki workspace identity");
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
    let (_dir, repo) = setup_repos();
    seed_main_file(repo.as_ref());
    inject_broken_structure(repo.as_ref());
    let wiki_root = repo.local_repo_workspace_root("wiki").expect("wiki root");
    std::fs::create_dir_all(&wiki_root).expect("wiki root");
    std::fs::write(wiki_root.join("untracked.md"), "must stay unscanned").expect("wiki file");

    let sync = SyncManager::new_checked(repo.clone()).expect("sync manager");
    sync.scan().expect("startup scan should skip broken repo");

    assert_eq!(
        std::fs::read_to_string(
            repo.local_repo_workspace_path("main", "notes/live.md")
                .expect("main doc path")
        )
        .expect("main doc"),
        "ok"
    );
    assert!(
        !repo
            .local_repo_workspace_path("wiki", "orphan.md")
            .expect("wiki orphan path")
            .exists()
    );
    assert!(sync.is_projection_degraded("wiki"));
    assert!(!sync.is_projection_degraded("main"));
    assert_eq!(
        sync.healthy_local_repo_names_for_execution()
            .expect("healthy repos"),
        vec![String::from("main")]
    );
    assert_eq!(
        sync.degraded_local_repo_names_for_execution()
            .expect("degraded repos"),
        vec![String::from("wiki")]
    );
    assert!(
        repo.list_pending_fs_in_local_repo("wiki")
            .unwrap()
            .is_empty()
    );
    assert!(
        sync.handle_fs_event(
            "wiki",
            repo.get_repo_info_for(None, Some("wiki"))
                .expect("wiki info lookup")
                .expect("wiki info")
                .uuid,
            "untracked.md"
        )
        .expect("ignored event")
        .is_empty()
    );
    assert!(
        repo.list_pending_fs_in_local_repo("wiki")
            .unwrap()
            .is_empty()
    );
}
