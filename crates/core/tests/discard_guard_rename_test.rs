use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, Op};
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::sync::SyncManager;
use std::sync::Arc;
use tempfile::TempDir;

mod common;

fn new_repo() -> (TempDir, Arc<RepoManager>) {
    let dir = TempDir::new().expect("create tempdir");
    let (repo, _repo_id) = common::init_cataloged_repo_with_depth(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )
    .expect("init cataloged repo");
    (dir, Arc::new(repo))
}

fn seed_ledger_doc(repo: &RepoManager) -> anyhow::Result<DocId> {
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
                    content: "ledger".into(),
                },
                1,
                repo.local_peer_id().clone(),
                seq,
                None,
                None,
            )
        },
    )?;
    Ok(doc_id)
}

#[test]
fn discard_renamed_pending_restores_canonical_path() -> anyhow::Result<()> {
    let (_dir, repo) = new_repo();
    let repo_name = repo.local_repo_name().to_string();
    let doc_id = seed_ledger_doc(&repo)?;
    let old_path = repo.local_repo_workspace_path(&repo_name, "notes/a.md")?;
    let new_path = repo.local_repo_workspace_path(&repo_name, "notes/b.md")?;
    std::fs::create_dir_all(old_path.parent().expect("parent"))?;
    std::fs::write(&new_path, "dirty")?;
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/b.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Renamed,
                content_hash: pending_fs::content_hash("dirty"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })?;

    let sync = SyncManager::new_checked(repo)?;
    sync.discard_pending_in_local_repo(&repo_name, "notes/b.md")?;

    assert_eq!(std::fs::read_to_string(old_path)?, "ledger");
    assert!(!new_path.exists());
    Ok(())
}

#[test]
fn repo_discard_renamed_pending_restores_canonical_path() -> anyhow::Result<()> {
    let (_dir, repo) = new_repo();
    let repo_name = repo.local_repo_name().to_string();
    let doc_id = seed_ledger_doc(&repo)?;
    let old_path = repo.local_repo_workspace_path(&repo_name, "notes/a.md")?;
    let new_path = repo.local_repo_workspace_path(&repo_name, "notes/b.md")?;
    std::fs::create_dir_all(old_path.parent().expect("parent"))?;
    std::fs::write(&new_path, "dirty")?;
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/b.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Renamed,
                content_hash: pending_fs::content_hash("dirty"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })?;

    repo.discard_pending_in_local_repo(&repo_name, "notes/b.md")?;

    assert_eq!(std::fs::read_to_string(old_path)?, "ledger");
    assert!(!new_path.exists());
    Ok(())
}

#[test]
fn discard_target_resolves_renamed_pending_by_doc_id() -> anyhow::Result<()> {
    let (_dir, repo) = new_repo();
    let repo_name = repo.local_repo_name().to_string();
    let (doc_id, _ops) = repo.apply_file_structure_in_local_repo(
        repo.local_repo_name(),
        "notes/a.md",
        None,
        "test",
    )?;
    let old_path = repo.local_repo_workspace_path(&repo_name, "notes/a.md")?;
    let new_path = repo.local_repo_workspace_path(&repo_name, "notes/b.md")?;
    std::fs::create_dir_all(old_path.parent().expect("parent"))?;
    std::fs::write(&new_path, "dirty")?;
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/b.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Renamed,
                content_hash: pending_fs::content_hash("dirty"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })?;

    let sync = SyncManager::new_checked(repo)?;
    let resolved = sync.discard_pending_target_in_local_repo(
        &repo_name,
        &ScPathTarget {
            path: "notes/a.md".into(),
            doc_id: Some(doc_id),
            domain: None,
        },
    )?;

    assert_eq!(resolved, "notes/b.md");
    assert_eq!(std::fs::read_to_string(old_path)?, "");
    assert!(!new_path.exists());
    Ok(())
}
