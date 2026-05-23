use super::super::dispatch::dispatch_batch;
use super::super::dispatch_test_support::{event_for, new_sync, rename_event};
use crate::source_control::ChangeStatus;

#[test]
fn dispatch_batch_ignores_event_paths_outside_repo_root() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    let outside = repo_root
        .parent()
        .expect("projection base")
        .join("outside.md");

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![event_for(outside)],
        None,
    )?;

    assert!(repo.list_pending_fs_in_local_repo(&repo_name)?.is_empty());
    Ok(())
}

#[test]
fn dispatch_batch_ignores_rename_pairs_that_leave_repo_root() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    let inside = repo_root.join("notes").join("draft.md");
    let outside = repo_root
        .parent()
        .expect("projection base")
        .join("outside.md");

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![rename_event(inside, outside)],
        None,
    )?;

    assert!(repo.list_pending_fs_in_local_repo(&repo_name)?.is_empty());
    Ok(())
}

#[test]
fn dispatch_batch_respects_deveignore_for_matching_markdown() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    std::fs::write(repo_root.join(".deveignore"), "ignored/*.md\n")?;
    let ignored = repo_root.join("ignored").join("scratch.md");
    std::fs::create_dir_all(ignored.parent().expect("parent"))?;
    std::fs::write(&ignored, "ignored")?;

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![event_for(ignored)],
        None,
    )?;

    assert!(repo.list_pending_fs_in_local_repo(&repo_name)?.is_empty());
    Ok(())
}

#[test]
fn dispatch_batch_respects_deveignore_during_dir_rescan() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    std::fs::write(repo_root.join(".deveignore"), "ignored/*.md\n")?;
    let ignored_dir = repo_root.join("ignored");
    std::fs::create_dir_all(&ignored_dir)?;
    std::fs::write(ignored_dir.join("scratch.md"), "ignored")?;

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![event_for(ignored_dir)],
        None,
    )?;

    assert!(repo.list_pending_fs_in_local_repo(&repo_name)?.is_empty());
    Ok(())
}

#[test]
fn dispatch_batch_allows_deveignore_non_matching_markdown() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    std::fs::write(repo_root.join(".deveignore"), "ignored/*.md\n")?;
    let kept = repo_root.join("notes").join("keep.md");
    std::fs::create_dir_all(kept.parent().expect("parent"))?;
    std::fs::write(&kept, "kept")?;

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![event_for(kept)],
        None,
    )?;

    let pending = repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert!(
        pending
            .iter()
            .any(|entry| { entry.path == "notes/keep.md" && entry.status == ChangeStatus::Added })
    );
    Ok(())
}
