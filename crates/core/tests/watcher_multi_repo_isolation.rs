//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

mod common;
mod watcher_test_support;

use deve_core::source_control::ChangeStatus;
use watcher_test_support::Harness;

#[test]
fn watcher_events_stay_repo_scoped() -> anyhow::Result<()> {
    let mut h = Harness::new(Some(("wiki", "urn:wiki")))?;
    std::fs::create_dir_all(h.workspace_path("main", "notes")?)?;
    std::fs::create_dir_all(h.workspace_path("wiki", "notes")?)?;
    h.start_watchers()?;

    let file = h.workspace_path("main", "notes/only-main.md")?;
    std::fs::create_dir_all(file.parent().expect("parent"))?;
    std::fs::write(&file, "main")?;

    h.wait_pending("main", "notes/only-main.md", ChangeStatus::Added)?;
    assert!(
        h.repo
            .list_pending_fs_in_local_repo(&h.repo_name("wiki"))?
            .is_empty()
    );
    Ok(())
}
