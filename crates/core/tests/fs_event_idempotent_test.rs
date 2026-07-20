use deve_core::models::{LedgerEntry, Op};
use deve_core::sync::SyncManager;
use tempfile::TempDir;

mod common;

fn new_repo() -> (TempDir, std::sync::Arc<deve_core::ledger::RepoManager>) {
    let dir = TempDir::new().expect("create tempdir");
    let (repo, _repo_id) = common::init_cataloged_repo_with_depth(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )
    .expect("init cataloged repo");
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
        repo.local_peer_id().clone(),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: "hello".into(),
                },
                1,
                repo.local_peer_id().clone(),
                seq,
                None,
                None,
            )
        },
    )?;

    let sync = SyncManager::new_checked(repo.clone())?;
    sync.persist_doc(doc_id)?;
    let repo_id = repo
        .get_repo_info_for(None, Some(repo.local_repo_name()))?
        .expect("repo info")
        .uuid;
    assert!(
        sync.handle_fs_event(repo.local_repo_name(), repo_id, "notes/a.md")?
            .is_empty()
    );

    let file = repo.local_repo_workspace_path(repo.local_repo_name(), "notes/a.md")?;
    std::fs::write(&file, "dirty")?;
    let first = sync.handle_fs_event(repo.local_repo_name(), repo_id, "notes/a.md")?;
    let second = sync.handle_fs_event(repo.local_repo_name(), repo_id, "notes/a.md")?;

    assert!(!first.is_empty());
    assert!(second.is_empty());
    let pending = repo.list_pending_fs_in_local_repo(repo.local_repo_name())?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "notes/a.md");
    Ok(())
}
