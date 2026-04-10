use super::dispatch::dispatch_batch;
use crate::ledger::RepoManager;
use crate::sync::SyncManager;
use notify_debouncer_full::{
    DebouncedEvent,
    notify::{
        Event, EventKind,
        event::{ModifyKind, RenameMode},
    },
};
use std::sync::Arc;
use std::time::Instant;

fn new_sync() -> anyhow::Result<(
    tempfile::TempDir,
    Arc<RepoManager>,
    Arc<SyncManager>,
    String,
    crate::models::RepoId,
    std::path::PathBuf,
)> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let vault = dir.path().join("vault");
    std::fs::create_dir_all(&vault)?;
    let mut repo = RepoManager::init(&ledger, 10, None, None)?;
    repo.set_vault_root_checked(&vault)?;
    let repo = Arc::new(repo);
    let sync = Arc::new(SyncManager::new_checked(repo.clone(), vault)?);
    let repo_name = repo.local_repo_name().to_string();
    let repo_id = repo
        .get_repo_info_for(None, Some(&repo_name))?
        .expect("repo info")
        .uuid;
    let repo_root = repo.local_repo_workspace_root(&repo_name)?;
    std::fs::create_dir_all(&repo_root)?;
    Ok((dir, repo, sync, repo_name, repo_id, repo_root))
}

#[test]
fn dispatch_batch_ignores_event_paths_outside_repo_root() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    let outside = repo_root.parent().expect("vault root").join("outside.md");

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![DebouncedEvent::new(
            Event {
                kind: EventKind::Any,
                paths: vec![outside],
                attrs: Default::default(),
            },
            Instant::now(),
        )],
        None,
    )?;

    assert!(repo.list_pending_fs_in_local_repo(&repo_name)?.is_empty());
    Ok(())
}

#[test]
fn dispatch_batch_ignores_rename_pairs_that_leave_repo_root() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    let inside = repo_root.join("notes").join("draft.md");
    let outside = repo_root.parent().expect("vault root").join("outside.md");

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![DebouncedEvent::new(
            Event {
                kind: EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
                paths: vec![inside, outside],
                attrs: Default::default(),
            },
            Instant::now(),
        )],
        None,
    )?;

    assert!(repo.list_pending_fs_in_local_repo(&repo_name)?.is_empty());
    Ok(())
}
