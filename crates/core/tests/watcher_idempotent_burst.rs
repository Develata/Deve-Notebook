//! plan_ref:
//!   - 04_storage.md §Watcher Architecture

mod watcher_test_support;

use deve_core::ledger::schema::PENDING_FS_OPS;
use deve_core::source_control::ChangeStatus;
use watcher_test_support::Harness;

#[test]
fn watcher_burst_keeps_pending_row_byte_stable() -> anyhow::Result<()> {
    let mut h = Harness::new(None)?;
    h.commit_doc("main", "notes/live.md", "base")?;
    h.start_watchers()?;

    let file = h.dir.path().join("vault/main/notes/live.md");
    for _ in 0..100 {
        std::fs::write(&file, "dirty")?;
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    h.wait_pending("main", "notes/live.md", ChangeStatus::Modified)?;
    let first = pending_bytes(&h, "notes/live.md")?;

    for _ in 0..20 {
        std::fs::write(&file, "dirty")?;
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    h.wait_pending("main", "notes/live.md", ChangeStatus::Modified)?;
    let second = pending_bytes(&h, "notes/live.md")?;
    assert_eq!(first, second);
    Ok(())
}

fn pending_bytes(h: &Harness, path: &str) -> anyhow::Result<Vec<u8>> {
    h.repo.run_on_local_repo("main", |db| {
        let read = db.begin_read()?;
        let table = read.open_table(PENDING_FS_OPS)?;
        table
            .get(path)?
            .map(|value| value.value().to_vec())
            .ok_or_else(|| anyhow::anyhow!("missing pending row: {path}"))
    })
}
