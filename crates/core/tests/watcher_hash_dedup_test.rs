//! DIFF-006: Watcher hash 去重 — 相同内容的重复写入不产生新 pending op。

use deve_core::models::{LedgerEntry, Op};
use deve_core::sync::SyncManager;
use tempfile::TempDir;

mod common;

fn new_repo() -> (TempDir, std::sync::Arc<deve_core::ledger::RepoManager>) {
    let dir = TempDir::new().expect("create tempdir");
    let (repo, _repo_id) =
        common::init_cataloged_repo(&dir.path().join("ledger"), &dir.path().join("notes"))
            .expect("init cataloged repo");
    (dir, std::sync::Arc::new(repo))
}

fn repo_id(repo: &deve_core::ledger::RepoManager, repo_name: &str) -> anyhow::Result<uuid::Uuid> {
    Ok(repo
        .get_repo_info_for(None, Some(repo_name))?
        .expect("repo info")
        .uuid)
}

#[test]
fn same_content_write_deduped_by_hash() -> anyhow::Result<()> {
    let (_dir, repo) = new_repo();
    let name = repo.local_repo_name().to_string();
    let (doc_id, _ops) =
        repo.apply_file_structure_in_local_repo(&name, "dedup.md", None, "test")?;
    let peer = repo.local_peer_id().clone();
    repo.append_generated_op_in_local_repo(&name, doc_id, peer.clone(), |seq| {
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "original".into(),
            },
            1,
            peer.clone(),
            seq,
            None,
            None,
        )
    })?;

    let sync = SyncManager::new_checked(repo.clone())?;
    sync.persist_doc(doc_id)?;
    let repo_id = repo_id(&repo, &name)?;

    // File on disk matches projection — no pending change
    let events = sync.handle_fs_event(&name, repo_id, "dedup.md")?;
    assert!(events.is_empty(), "no change when content matches");

    // Write different content — should produce pending
    let file = repo.local_repo_workspace_path(&name, "dedup.md")?;
    std::fs::write(&file, "modified")?;
    let first = sync.handle_fs_event(&name, repo_id, "dedup.md")?;
    assert!(
        !first.is_empty(),
        "first modification should produce change"
    );

    // Write same modified content again — hash dedup
    let second = sync.handle_fs_event(&name, repo_id, "dedup.md")?;
    assert!(second.is_empty(), "repeated same content is deduped");

    // Only 1 pending op should exist
    let pending = repo.list_pending_fs_in_local_repo(&name)?;
    assert_eq!(pending.len(), 1);
    Ok(())
}

#[test]
fn revert_to_original_clears_pending() -> anyhow::Result<()> {
    let (_dir, repo) = new_repo();
    let name = repo.local_repo_name().to_string();
    let (doc_id, _ops) =
        repo.apply_file_structure_in_local_repo(&name, "revert.md", None, "test")?;
    let peer = repo.local_peer_id().clone();
    repo.append_generated_op_in_local_repo(&name, doc_id, peer.clone(), |seq| {
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "baseline".into(),
            },
            1,
            peer.clone(),
            seq,
            None,
            None,
        )
    })?;

    let sync = SyncManager::new_checked(repo.clone())?;
    sync.persist_doc(doc_id)?;
    let repo_id = repo_id(&repo, &name)?;

    // Modify then revert
    let file = repo.local_repo_workspace_path(&name, "revert.md")?;
    std::fs::write(&file, "dirty")?;
    let changed = sync.handle_fs_event(&name, repo_id, "revert.md")?;
    assert!(!changed.is_empty());

    std::fs::write(&file, "baseline")?;
    let reverted = sync.handle_fs_event(&name, repo_id, "revert.md")?;
    assert!(!reverted.is_empty(), "revert event fires");

    // After revert, pending should be empty (content matches projection)
    let pending = repo.list_pending_fs_in_local_repo(&name)?;
    assert!(pending.is_empty(), "revert clears pending");
    Ok(())
}
