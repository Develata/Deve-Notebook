//! plan_ref:
//!   - 04_storage.md §Inode/DocId Mapping & Watcher Service
//!   - 04_storage.md §Watcher Architecture

mod watcher_test_support;

use deve_core::source_control::ChangeStatus;
use watcher_test_support::Harness;

#[test]
fn watcher_events_stay_repo_scoped() -> anyhow::Result<()> {
    let mut h = Harness::new(Some(("wiki", "urn:wiki")))?;
    std::fs::create_dir_all(h.dir.path().join("vault/main/notes"))?;
    std::fs::create_dir_all(h.dir.path().join("vault/wiki/notes"))?;
    h.start_watchers()?;

    let file = h.dir.path().join("vault/main/notes/only-main.md");
    std::fs::create_dir_all(file.parent().expect("parent"))?;
    std::fs::write(&file, "main")?;

    h.wait_pending("main", "notes/only-main.md", ChangeStatus::Added)?;
    assert!(h.repo.list_pending_fs_in_local_repo("wiki")?.is_empty());
    Ok(())
}
