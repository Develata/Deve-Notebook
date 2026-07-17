use super::super::WatcherCallback;
use super::super::dispatch::dispatch_batch;
use super::super::dispatch_test_support::{
    assert_fs_message, commit_doc, event_for, new_sync, removed_dir_event, rename_event,
};
use crate::source_control::ChangeStatus;
use std::sync::{Arc, Mutex};

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
        vec![event_for(&repo_root, ignored)],
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
        vec![event_for(&repo_root, ignored_dir)],
        None,
    )?;

    assert!(repo.list_pending_fs_in_local_repo(&repo_name)?.is_empty());
    Ok(())
}

#[test]
fn dispatch_batch_scans_repo_root_directory_event() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    std::fs::write(repo_root.join("root.md"), "root")?;

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![event_for(&repo_root, repo_root.clone())],
        None,
    )?;

    let pending = repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "root.md");
    assert_eq!(pending[0].status, ChangeStatus::Added);
    Ok(())
}

#[test]
fn dispatch_batch_rescans_removed_directory_after_path_disappears() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    commit_doc(&repo, &sync, &repo_name, "notes/removed/a.md", "base")?;
    let removed = repo_root.join("notes").join("removed");
    let messages = Arc::new(Mutex::new(Vec::new()));
    let callback_messages = messages.clone();
    let callback: WatcherCallback = Arc::new(move |message| {
        callback_messages
            .lock()
            .expect("messages lock")
            .push(message);
    });

    std::fs::remove_dir_all(&removed)?;

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![removed_dir_event(&repo_root, removed)],
        Some(&callback),
    )?;

    let pending = repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert!(pending.iter().any(|entry| {
        entry.path == "notes/removed/a.md" && entry.status == ChangeStatus::Deleted
    }));
    let messages = messages.lock().expect("messages lock");
    assert_eq!(messages.len(), 1);
    assert_fs_message(&messages[0], "notes/removed", "dir_changed");
    Ok(())
}

#[test]
fn dispatch_batch_coalesces_removed_directories_to_one_refresh() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    commit_doc(&repo, &sync, &repo_name, "one/a.md", "a")?;
    commit_doc(&repo, &sync, &repo_name, "two/b.md", "b")?;
    let one = repo_root.join("one");
    let two = repo_root.join("two");
    std::fs::remove_dir_all(&one)?;
    std::fs::remove_dir_all(&two)?;
    let messages = Arc::new(Mutex::new(Vec::new()));
    let callback_messages = messages.clone();
    let callback: WatcherCallback = Arc::new(move |message| {
        callback_messages
            .lock()
            .expect("messages lock")
            .push(message);
    });

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![
            removed_dir_event(&repo_root, one),
            removed_dir_event(&repo_root, two),
        ],
        Some(&callback),
    )?;

    let pending = repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert!(pending.iter().any(|entry| entry.path == "one/a.md"));
    assert!(pending.iter().any(|entry| entry.path == "two/b.md"));
    assert_eq!(messages.lock().expect("messages lock").len(), 1);
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
        vec![event_for(&repo_root, kept)],
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

#[test]
fn dispatch_batch_degrades_untracked_rename_pair_to_added_path() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    let old = repo_root.join("notes").join("draft.md");
    let new = repo_root.join("notes").join("renamed.md");
    std::fs::create_dir_all(old.parent().expect("parent"))?;
    std::fs::write(&old, "draft")?;
    std::fs::rename(&old, &new)?;

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![rename_event(&repo_root, old, new)],
        None,
    )?;

    let pending = repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "notes/renamed.md");
    assert_eq!(pending[0].status, ChangeStatus::Added);
    Ok(())
}

#[test]
fn dispatch_batch_suppresses_self_write_rename_pair() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    commit_doc(&repo, &sync, &repo_name, "notes/a.md", "base")?;
    let old = repo_root.join("notes").join("a.md");
    let new = repo_root.join("notes").join("b.md");

    repo.record_projection_delete(&repo_name, "notes/a.md");
    repo.record_projection_write(&repo_name, "notes/b.md", "base");
    std::fs::rename(&old, &new)?;

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![rename_event(&repo_root, old, new)],
        None,
    )?;

    assert!(
        repo.list_pending_fs_in_local_repo(&repo_name)?.is_empty(),
        "projection self-write rename pair must not become pending external change"
    );
    Ok(())
}

#[test]
fn dispatch_batch_degrades_rename_from_non_markdown_to_markdown_as_add() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    let old = repo_root.join("notes").join("draft.txt");
    let new = repo_root.join("notes").join("draft.md");
    std::fs::create_dir_all(old.parent().expect("parent"))?;
    std::fs::write(&old, "draft")?;
    std::fs::rename(&old, &new)?;

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![rename_event(&repo_root, old, new)],
        None,
    )?;

    let pending = repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "notes/draft.md");
    assert_eq!(pending[0].status, ChangeStatus::Added);
    Ok(())
}

#[test]
fn dispatch_batch_degrades_rename_from_ignored_to_tracked_as_add() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    std::fs::write(repo_root.join(".deveignore"), "ignored/*.md\n")?;
    let ignored = repo_root.join("ignored").join("draft.md");
    let tracked = repo_root.join("notes").join("draft.md");
    std::fs::create_dir_all(ignored.parent().expect("ignored parent"))?;
    std::fs::create_dir_all(tracked.parent().expect("tracked parent"))?;
    std::fs::write(&ignored, "promoted")?;
    std::fs::rename(&ignored, &tracked)?;

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![rename_event(&repo_root, ignored, tracked)],
        None,
    )?;

    let pending = repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "notes/draft.md");
    assert_eq!(pending[0].status, ChangeStatus::Added);
    Ok(())
}

#[test]
fn dispatch_batch_degrades_rename_from_tracked_to_ignored_as_delete() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    std::fs::write(repo_root.join(".deveignore"), "ignored/*.md\n")?;
    let doc_id = commit_doc(&repo, &sync, &repo_name, "notes/a.md", "base")?;
    let old = repo_root.join("notes").join("a.md");
    let ignored = repo_root.join("ignored").join("a.md");
    std::fs::create_dir_all(ignored.parent().expect("ignored parent"))?;
    std::fs::rename(&old, &ignored)?;

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![rename_event(&repo_root, old, ignored)],
        None,
    )?;

    let pending = repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "notes/a.md");
    assert_eq!(pending[0].status, ChangeStatus::Deleted);
    assert_eq!(pending[0].doc_id, Some(doc_id));
    Ok(())
}
