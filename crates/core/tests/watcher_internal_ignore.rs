//! plan_ref:
//!   - 04_storage.md §8. Watcher Contract

mod watcher_test_support;

use watcher_test_support::Harness;

#[test]
fn watcher_ignores_internal_notegit_paths() -> anyhow::Result<()> {
    let mut h = Harness::new(None)?;
    h.start_watchers()?;

    let internal = h.dir.path().join("vault/main/.notegit/x.md");
    std::fs::create_dir_all(internal.parent().expect("parent"))?;
    std::fs::write(&internal, "tmp")?;

    wait_no_pending(&h, ".notegit/x.md")?;
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
