//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

mod common;
mod watcher_test_support;

use deve_core::sync::watcher::{
    RepoWatcherHandle, RepoWatcherStart, RepoWatcherWorkerState, WatcherFailureKind,
    WatcherFailurePhase,
};
use std::time::Duration;
use watcher_test_support::Harness;

#[test]
fn repo_watcher_handle_reports_identity_and_restarts_after_shutdown() -> anyhow::Result<()> {
    let h = Harness::new(None)?;
    let repo_name = h.repo.local_repo_name().to_string();
    let repo_id = h
        .repo
        .get_repo_info_for(None, Some(&repo_name))?
        .expect("repo info")
        .uuid;
    let first = RepoWatcherHandle::start(RepoWatcherStart::new(
        h.sync.clone(),
        repo_id,
        &repo_name,
        1,
    ))?;

    assert_eq!(first.repo_id(), repo_id);
    assert_eq!(first.generation(), 1);
    let snapshot = first.snapshot();
    assert_eq!(snapshot.repo_id(), repo_id);
    assert_eq!(snapshot.generation(), 1);
    assert!(matches!(
        snapshot.worker_state(),
        RepoWatcherWorkerState::Running
    ));

    first.shutdown()?;
    let restarted =
        RepoWatcherHandle::start(RepoWatcherStart::new(h.sync.clone(), repo_id, repo_name, 2))?;
    assert_eq!(restarted.generation(), 2);
    restarted.shutdown()?;
    Ok(())
}

#[test]
fn watcher_rejects_zero_debounce_window() -> anyhow::Result<()> {
    let h = Harness::new(None)?;
    let repo_name = h.repo.local_repo_name().to_string();

    let err = match RepoWatcherHandle::start(
        RepoWatcherStart::resolve(h.sync.clone(), repo_name, 1)?.with_debounce(Duration::ZERO),
    ) {
        Ok(_) => panic!("zero debounce must fail closed before watcher start"),
        Err(error) => error,
    };

    assert_eq!(err.failure().phase, WatcherFailurePhase::Prepare);
    assert_eq!(err.failure().kind, WatcherFailureKind::Configuration);
    Ok(())
}

#[test]
fn watcher_drop_is_a_synchronous_cleanup_safety_net() -> anyhow::Result<()> {
    let h = Harness::new(None)?;
    let repo_name = h.repo.local_repo_name().to_string();
    let handle =
        RepoWatcherHandle::start(RepoWatcherStart::resolve(h.sync.clone(), &repo_name, 1)?)?;

    drop(handle);
    let path = h.workspace_path(&repo_name, "notes/after-drop.md")?;
    std::fs::create_dir_all(path.parent().expect("post-drop parent"))?;
    std::fs::write(path, "after drop")?;
    std::thread::sleep(Duration::from_millis(700));

    assert!(h.repo.list_pending_fs_in_local_repo(&repo_name)?.is_empty());
    Ok(())
}
