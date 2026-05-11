//! plan_ref:
//!   - 04_storage#watcher-contract

mod common;
mod watcher_test_support;

use watcher_test_support::Harness;

#[test]
fn projection_writeback_events_are_suppressed() -> anyhow::Result<()> {
    let mut h = Harness::new(None)?;
    let doc_id = h.commit_doc("main", "notes/live.md", "base")?;
    h.start_watchers()?;

    h.sync.persist_doc_in_local_repo("main", doc_id)?;
    wait_no_pending(&h, "notes/live.md")?;
    assert!(h.repo.list_pending_fs_in_local_repo("main")?.is_empty());
    Ok(())
}

fn wait_no_pending(h: &Harness, path: &str) -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_millis(700) {
        if h.repo
            .list_pending_fs_in_local_repo("main")?
            .iter()
            .all(|entry| entry.path != path)
        {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    anyhow::bail!("timeout waiting for no pending entry: {path}");
}
