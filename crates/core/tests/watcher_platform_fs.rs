//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!
//! Cross-platform real-filesystem watcher evidence. CI runs this test target on
//! Windows and Linux with one test thread to prevent tests from competing for
//! filesystem watcher resources. Assertions depend only on final state.

mod common;
mod watcher_test_support;

use anyhow::Context;
use deve_core::source_control::{ChangeStatus, pending_fs};
use deve_core::sync::watcher::{self, DEFAULT_DEBOUNCE, RepoWatcherWorkerState};
use std::io::Write;
use std::time::Duration;
use watcher_test_support::Harness;

#[test]
fn watcher_atomic_replace_records_single_final_candidate() -> anyhow::Result<()> {
    let mut h = Harness::new(None)?;
    let doc_id = h.commit_doc("main", "notes/live.md", "base")?;
    h.start_watchers()?;
    let before_head = ledger_head(&h)?;
    let target = h.workspace_path("main", "notes/live.md")?;
    let parent = target.parent().context("watcher target parent")?;
    let mut replacement = tempfile::NamedTempFile::new_in(parent)?;
    replacement.write_all(b"final")?;
    replacement.as_file_mut().sync_all()?;
    replacement.persist(&target).map_err(|error| error.error)?;

    let change = h.wait_pending("main", "notes/live.md", ChangeStatus::Modified)?;
    assert_eq!(change.doc_id, Some(doc_id));
    assert_eq!(change.renamed_from, None);
    let pending = h
        .repo
        .run_on_local_repo("main", |db| pending_fs::get(db, "notes/live.md"))?;
    let pending = pending.context("atomic replace pending row")?;
    assert_eq!(pending.content_hash, pending_fs::content_hash("final"));
    assert_eq!(h.repo.list_pending_fs_in_local_repo("main")?.len(), 1);
    assert_eq!(ledger_head(&h)?, before_head);
    Ok(())
}

#[test]
fn watcher_directory_removal_rescans_tracked_descendants() -> anyhow::Result<()> {
    let mut h = Harness::new(None)?;
    h.commit_doc("main", "notes/removed/a.md", "a")?;
    h.commit_doc("main", "notes/removed/b.md", "b")?;
    h.start_watchers()?;
    let before_head = ledger_head(&h)?;

    std::fs::remove_dir_all(h.workspace_path("main", "notes/removed")?)?;

    h.wait_pending("main", "notes/removed/a.md", ChangeStatus::Deleted)?;
    h.wait_pending("main", "notes/removed/b.md", ChangeStatus::Deleted)?;
    assert_eq!(ledger_head(&h)?, before_head);
    Ok(())
}

#[test]
fn watcher_stop_prevents_post_stop_delivery() -> anyhow::Result<()> {
    let h = Harness::new(None)?;
    let repo_name = h.repo.local_repo_name().to_string();
    let handle = watcher::RepoWatcherHandle::start(watcher::RepoWatcherStart::resolve(
        h.sync.clone(),
        &repo_name,
        1,
    )?)?;
    handle.shutdown()?;

    let path = h.workspace_path(&repo_name, "notes/after-stop.md")?;
    std::fs::create_dir_all(path.parent().context("post-stop parent")?)?;
    std::fs::write(path, "after stop")?;
    std::thread::sleep(DEFAULT_DEBOUNCE * 3 + Duration::from_millis(200));

    assert!(h.repo.list_pending_fs_in_local_repo(&repo_name)?.is_empty());
    Ok(())
}

#[test]
fn watcher_capture_first_startup_reaches_running_with_preexisting_and_post_start_changes()
-> anyhow::Result<()> {
    let h = Harness::new(None)?;
    let repo_name = h.repo.local_repo_name().to_string();
    let pre_start = h.workspace_path(&repo_name, "notes/pre-start.md")?;
    std::fs::create_dir_all(pre_start.parent().context("pre-start parent")?)?;
    std::fs::write(&pre_start, "pre-start")?;

    let handle = watcher::RepoWatcherHandle::start(watcher::RepoWatcherStart::resolve(
        h.sync.clone(),
        &repo_name,
        1,
    )?)?;
    assert!(matches!(
        handle.snapshot().worker_state(),
        RepoWatcherWorkerState::Running
    ));
    h.wait_pending(&repo_name, "notes/pre-start.md", ChangeStatus::Added)?;

    let post_start = h.workspace_path(&repo_name, "notes/post-start.md")?;
    std::fs::write(post_start, "post-start")?;
    h.wait_pending(&repo_name, "notes/post-start.md", ChangeStatus::Added)?;
    handle.shutdown()?;
    Ok(())
}

fn ledger_head(h: &Harness) -> anyhow::Result<u64> {
    h.repo
        .run_on_local_repo("main", deve_core::ledger::range::get_max_seq)
}
