use deve_core::models::{LedgerEntry, Op, PeerId};
use deve_core::sync::SyncManager;
use tempfile::TempDir;

fn new_repo() -> (TempDir, std::sync::Arc<deve_core::ledger::RepoManager>) {
    let dir = TempDir::new().expect("create tempdir");
    let mut repo = deve_core::ledger::RepoManager::init(dir.path().join("ledger"), 10, None, None)
        .expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    (dir, std::sync::Arc::new(repo))
}

#[test]
fn repeated_same_fs_modify_event_is_noop_after_first_pending_update() -> anyhow::Result<()> {
    let (_dir, repo) = new_repo();
    let (doc_id, _ops) = repo.apply_file_structure_in_local_repo(
        repo.local_repo_name(),
        "notes/a.md",
        None,
        "test",
    )?;
    repo.append_generated_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        PeerId::new("local"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: "hello".into(),
                },
                1,
                PeerId::new("local"),
                seq,
                None,
                None,
            )
        },
    )?;

    let sync = SyncManager::new_checked(repo.clone())?;
    sync.persist_doc(doc_id)?;
    let repo_id = repo
        .get_repo_info_for(None, Some("default"))?
        .expect("repo info")
        .uuid;
    assert!(
        sync.handle_fs_event("default", repo_id, "notes/a.md")?
            .is_empty()
    );

    let file = repo.local_repo_workspace_path("default", "notes/a.md")?;
    std::fs::write(&file, "dirty")?;
    let first = sync.handle_fs_event("default", repo_id, "notes/a.md")?;
    let second = sync.handle_fs_event("default", repo_id, "notes/a.md")?;

    assert!(!first.is_empty());
    assert!(second.is_empty());
    let pending = repo.list_pending_fs_in_local_repo(repo.local_repo_name())?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "notes/a.md");
    Ok(())
}
