use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, NodeId, Op, PeerId, StructureOp};
use deve_core::sync::SyncManager;
use std::sync::Arc;
use tempfile::TempDir;

mod common;

/// Returns the primary manager plus both repos' canonical execution names
/// (their RepoId strings). `init_cataloged_repo` already establishes each
/// workspace identity marker during the production creation choreography, so the
/// scan's identity gate is exercised against legitimately-owned workspaces.
fn setup_repos() -> (TempDir, Arc<RepoManager>, String, String) {
    let dir = TempDir::new().expect("tempdir");
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let (main, main_id) =
        common::init_cataloged_repo_with_url(&ledger, &projection_base, "urn:main").expect("main");
    let (_wiki, wiki_id) =
        common::init_cataloged_repo_with_url(&ledger, &projection_base, "urn:wiki").expect("wiki");
    (
        dir,
        Arc::new(main),
        main_id.to_string(),
        wiki_id.to_string(),
    )
}

fn seed_main_file(repo: &RepoManager, repo_name: &str) {
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo(repo_name, "notes/live.md", None, "test")
        .expect("create main doc");
    let peer = repo.local_peer_id().clone();
    repo.append_generated_op_in_local_repo(repo_name, doc_id, peer.clone(), |seq| {
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "ok".into(),
            },
            1,
            peer.clone(),
            seq,
            None,
            None,
        )
    })
    .expect("append main content");
}

fn inject_broken_structure(repo: &RepoManager, repo_name: &str) {
    let doc_id = DocId::new();
    common::append_unvalidated_local_op(
        repo,
        repo_name,
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
    let (_dir, repo, main_execution_name, wiki_execution_name) = setup_repos();
    seed_main_file(repo.as_ref(), &main_execution_name);
    inject_broken_structure(repo.as_ref(), &wiki_execution_name);
    let wiki_root = repo
        .local_repo_workspace_root(&wiki_execution_name)
        .expect("wiki root");
    std::fs::create_dir_all(&wiki_root).expect("wiki root");
    std::fs::write(wiki_root.join("untracked.md"), "must stay unscanned").expect("wiki file");

    let sync = SyncManager::new_checked(repo.clone()).expect("sync manager");
    sync.scan().expect("startup scan should skip broken repo");

    assert_eq!(
        std::fs::read_to_string(
            repo.local_repo_workspace_path(&main_execution_name, "notes/live.md")
                .expect("main doc path")
        )
        .expect("main doc"),
        "ok"
    );
    assert!(
        !repo
            .local_repo_workspace_path(&wiki_execution_name, "orphan.md")
            .expect("wiki orphan path")
            .exists()
    );
    assert!(sync.is_projection_degraded(&wiki_execution_name));
    assert!(!sync.is_projection_degraded(&main_execution_name));
    assert_eq!(
        sync.healthy_local_repo_names_for_execution()
            .expect("healthy repos"),
        vec![main_execution_name]
    );
    assert_eq!(
        sync.degraded_local_repo_names_for_execution()
            .expect("degraded repos"),
        vec![wiki_execution_name.clone()]
    );
    assert!(
        repo.list_pending_fs_in_local_repo(&wiki_execution_name)
            .unwrap()
            .is_empty()
    );
    assert!(
        sync.handle_fs_event(
            &wiki_execution_name,
            repo.get_repo_info_for(None, Some(&wiki_execution_name))
                .expect("wiki info lookup")
                .expect("wiki info")
                .uuid,
            "untracked.md"
        )
        .expect("ignored event")
        .is_empty()
    );
    assert!(
        repo.list_pending_fs_in_local_repo(&wiki_execution_name)
            .unwrap()
            .is_empty()
    );
}
