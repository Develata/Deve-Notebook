//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!
//! STORE-007: watcher maps create/modify/delete into pending_fs_ops only.

mod common;
mod watcher_test_support;

use anyhow::Context;
use deve_core::source_control::ChangeStatus;
use deve_core::sync::watcher::DEFAULT_DEBOUNCE;
use watcher_test_support::Harness;

#[test]
fn watcher_records_create_modify_delete_candidates() -> anyhow::Result<()> {
    let mut h = Harness::new(None).context("create watcher test harness")?;
    h.commit_doc("main", "notes/live.md", "base")
        .context("commit initial tracked document")?;
    h.start_watchers().context("start repo watchers")?;

    let created = h.workspace_path("main", "notes/new.md")?;
    std::fs::create_dir_all(created.parent().expect("parent"))?;
    std::fs::write(&created, "new")?;
    h.wait_pending("main", "notes/new.md", ChangeStatus::Added)
        .context("observe created document")?;

    let tracked = h.workspace_path("main", "notes/live.md")?;
    std::fs::write(&tracked, "dirty")?;
    h.wait_pending("main", "notes/live.md", ChangeStatus::Modified)
        .context("observe modified document")?;
    // Keep modify and delete in separate debounce windows; some backends coalesce
    // same-path write/remove bursts and make the delete edge unobservable.
    std::thread::sleep(DEFAULT_DEBOUNCE * 2);

    std::fs::remove_file(&tracked)?;
    h.wait_pending("main", "notes/live.md", ChangeStatus::Deleted)
        .context("observe deleted document")?;
    Ok(())
}
