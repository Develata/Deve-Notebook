//! plan_ref:
//!   - 04_storage#watcher-contract
//!
//! POS-005: `.deveignore` applies to watcher events and startup scan.
//! STORE-007: ignored directory scans do not create pending_fs_ops.

mod common;
mod watcher_test_support;

use deve_core::source_control::ChangeStatus;
use std::path::PathBuf;
use watcher_test_support::Harness;

#[test]
fn watcher_ignores_internal_notegit_paths() -> anyhow::Result<()> {
    let mut h = Harness::new(None)?;
    let root = ensure_workspace_root(&h)?;
    h.start_watchers()?;

    let internal = root.join(".notegit/x.md");
    std::fs::create_dir_all(internal.parent().expect("parent"))?;
    std::fs::write(&internal, "tmp")?;

    wait_no_pending(&h, ".notegit/x.md")?;
    assert!(h.repo.list_pending_fs_in_local_repo("main")?.is_empty());
    Ok(())
}

#[test]
fn watcher_ignores_internal_git_paths() -> anyhow::Result<()> {
    let mut h = Harness::new(None)?;
    let root = ensure_workspace_root(&h)?;
    h.start_watchers()?;

    let internal = root.join(".git/objects/x.md");
    std::fs::create_dir_all(internal.parent().expect("parent"))?;
    std::fs::write(&internal, "tmp")?;

    wait_no_pending(&h, ".git/objects/x.md")?;
    assert!(h.repo.list_pending_fs_in_local_repo("main")?.is_empty());
    Ok(())
}

#[test]
fn watcher_allows_notegit_backup_sibling_path() -> anyhow::Result<()> {
    let mut h = Harness::new(None)?;
    let root = ensure_workspace_root(&h)?;
    h.start_watchers()?;

    let sibling = root.join(".notegit-backup/x.md");
    std::fs::create_dir_all(sibling.parent().expect("parent"))?;
    std::fs::write(&sibling, "backup sibling")?;

    h.wait_pending("main", ".notegit-backup/x.md", ChangeStatus::Added)?;
    Ok(())
}

#[test]
fn watcher_respects_deveignore_for_matching_markdown() -> anyhow::Result<()> {
    let mut h = Harness::new(None)?;
    let root = ensure_workspace_root(&h)?;
    std::fs::write(root.join(".deveignore"), "ignored/*.md\n")?;
    h.start_watchers()?;

    let ignored = root.join("ignored/scratch.md");
    std::fs::create_dir_all(ignored.parent().expect("parent"))?;
    std::fs::write(&ignored, "ignored")?;

    wait_no_pending(&h, "ignored/scratch.md")?;
    assert!(h.repo.list_pending_fs_in_local_repo("main")?.is_empty());
    Ok(())
}

#[test]
fn watcher_allows_deveignore_non_matching_markdown() -> anyhow::Result<()> {
    let mut h = Harness::new(None)?;
    let root = ensure_workspace_root(&h)?;
    std::fs::write(root.join(".deveignore"), "ignored/*.md\n")?;
    h.start_watchers()?;

    let kept = root.join("notes/keep.md");
    std::fs::create_dir_all(kept.parent().expect("parent"))?;
    std::fs::write(&kept, "kept")?;

    h.wait_pending("main", "notes/keep.md", ChangeStatus::Added)?;
    Ok(())
}

#[test]
fn watcher_startup_scan_respects_deveignore() -> anyhow::Result<()> {
    let mut h = Harness::new(None)?;
    let root = ensure_workspace_root(&h)?;
    std::fs::write(root.join(".deveignore"), "ignored/*.md\n")?;
    let ignored = root.join("ignored/preexisting.md");
    std::fs::create_dir_all(ignored.parent().expect("parent"))?;
    std::fs::write(&ignored, "ignored")?;

    h.start_watchers()?;

    wait_no_pending(&h, "ignored/preexisting.md")?;
    assert!(h.repo.list_pending_fs_in_local_repo("main")?.is_empty());
    Ok(())
}

fn ensure_workspace_root(h: &Harness) -> anyhow::Result<PathBuf> {
    let root = h.workspace_root("main")?;
    std::fs::create_dir_all(&root)?;
    Ok(root)
}

fn wait_no_pending(h: &Harness, path: &str) -> anyhow::Result<()> {
    std::thread::sleep(std::time::Duration::from_millis(700));
    if h.repo
        .list_pending_fs_in_local_repo("main")?
        .iter()
        .all(|entry| entry.path != path)
    {
        return Ok(());
    }
    anyhow::bail!("unexpected pending entry: {path}");
}
