//! plan_ref:
//!   - 04_storage.md §8. Watcher Contract

mod watcher_test_support;

use deve_core::source_control::ChangeStatus;
use watcher_test_support::Harness;

#[test]
fn watcher_records_create_modify_delete_candidates() -> anyhow::Result<()> {
    let mut h = Harness::new(None)?;
    h.commit_doc("main", "notes/live.md", "base")?;
    h.start_watchers()?;

    let created = h.dir.path().join("vault/main/notes/new.md");
    std::fs::create_dir_all(created.parent().expect("parent"))?;
    std::fs::write(&created, "new")?;
    h.wait_pending("main", "notes/new.md", ChangeStatus::Added)?;

    let tracked = h.dir.path().join("vault/main/notes/live.md");
    std::fs::write(&tracked, "dirty")?;
    h.wait_pending("main", "notes/live.md", ChangeStatus::Modified)?;

    std::fs::remove_file(&tracked)?;
    h.wait_pending("main", "notes/live.md", ChangeStatus::Deleted)?;
    Ok(())
}
