//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

mod common;
mod watcher_test_support;

use deve_core::source_control::ChangeStatus;
use deve_core::sync::watcher::DEFAULT_DEBOUNCE;
use std::time::Duration;
use watcher_test_support::Harness;

#[test]
fn projection_writeback_events_are_suppressed() -> anyhow::Result<()> {
    let mut h = Harness::new(None)?;
    let doc_id = h.commit_doc("main", "notes/live.md", "base")?;
    h.start_watchers()?;

    h.sync
        .persist_doc_in_local_repo(&h.repo_name("main"), doc_id)?;
    let sentinel = h.workspace_path("main", "notes/watcher-live.md")?;
    std::fs::write(&sentinel, "external")?;
    h.wait_pending("main", "notes/watcher-live.md", ChangeStatus::Added)?;
    std::thread::sleep(DEFAULT_DEBOUNCE * 3 + Duration::from_millis(200));

    let pending = h.repo.list_pending_fs_in_local_repo(&h.repo_name("main"))?;
    assert!(
        pending
            .iter()
            .any(|entry| entry.path == "notes/watcher-live.md")
    );
    assert!(pending.iter().all(|entry| entry.path != "notes/live.md"));
    Ok(())
}
