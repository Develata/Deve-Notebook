//! plan_ref:
//!   - 03_storage#watcher-contract

mod common;
mod watcher_test_support;

use deve_core::ledger::schema::PENDING_FS_OPS;
use deve_core::source_control::ChangeStatus;
use std::io::{Seek, SeekFrom, Write};
use std::time::{Duration, Instant};
use watcher_test_support::Harness;

#[test]
fn watcher_burst_keeps_pending_row_byte_stable() -> anyhow::Result<()> {
    let mut h = Harness::new(None)?;
    h.commit_doc("main", "notes/live.md", "base")?;
    h.start_watchers()?;

    let file = h.workspace_path("main", "notes/live.md")?;
    std::fs::write(&file, "dirty")?;
    h.wait_pending("main", "notes/live.md", ChangeStatus::Modified)?;
    let first = stable_pending_bytes(&h, "notes/live.md")?;

    for _ in 0..100 {
        overwrite_same_bytes(&file, b"dirty")?;
        std::thread::sleep(Duration::from_millis(5));
    }

    h.wait_pending("main", "notes/live.md", ChangeStatus::Modified)?;
    let second = stable_pending_bytes(&h, "notes/live.md")?;
    assert_eq!(first, second);

    for _ in 0..20 {
        overwrite_same_bytes(&file, b"dirty")?;
        std::thread::sleep(Duration::from_millis(10));
    }

    h.wait_pending("main", "notes/live.md", ChangeStatus::Modified)?;
    let third = stable_pending_bytes(&h, "notes/live.md")?;
    assert_eq!(first, third);
    Ok(())
}

fn overwrite_same_bytes(path: &std::path::Path, bytes: &[u8]) -> anyhow::Result<()> {
    let mut file = std::fs::OpenOptions::new().write(true).open(path)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(bytes)?;
    file.flush()?;
    Ok(())
}

fn stable_pending_bytes(h: &Harness, path: &str) -> anyhow::Result<Vec<u8>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last = pending_bytes(h, path)?;
    let mut stable_since = Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(25));
        let next = pending_bytes(h, path)?;
        if next == last {
            if stable_since.elapsed() >= Duration::from_millis(200) {
                return Ok(next);
            }
        } else {
            last = next;
            stable_since = Instant::now();
        }
        if Instant::now() >= deadline {
            anyhow::bail!("pending row did not become stable: {path}");
        }
    }
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
