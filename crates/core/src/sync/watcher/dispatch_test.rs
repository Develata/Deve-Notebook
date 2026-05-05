use super::dispatch::dispatch_batch;
use super::dispatch_test_support::{
    assert_fs_message, commit_doc, event_for, new_sync, rename_event,
};
use crate::ledger::REPO_METADATA;
use crate::source_control::ChangeStatus;
use std::sync::{Arc, Mutex};

#[test]
fn dispatch_batch_ignores_event_paths_outside_repo_root() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    let outside = repo_root.parent().expect("vault root").join("outside.md");

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
    let outside = repo_root.parent().expect("vault root").join("outside.md");

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
    let vault_root = repo_root.parent().expect("vault root");
    std::fs::write(vault_root.join(".deveignore"), "ignored/*.md\n")?;
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
    let vault_root = repo_root.parent().expect("vault root");
    std::fs::write(vault_root.join(".deveignore"), "ignored/*.md\n")?;
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
    let vault_root = repo_root.parent().expect("vault root");
    std::fs::write(vault_root.join(".deveignore"), "ignored/*.md\n")?;
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

#[test]
fn dispatch_batch_suppresses_duplicate_external_added_message() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    let added = repo_root.join("notes").join("external.md");
    std::fs::create_dir_all(added.parent().expect("parent"))?;
    std::fs::write(&added, "external")?;

    let messages = Arc::new(Mutex::new(Vec::new()));
    let callback_messages = messages.clone();
    let callback: super::WatcherCallback = Arc::new(move |msg| {
        callback_messages.lock().expect("messages lock").push(msg);
    });

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![event_for(added.clone())],
        Some(&callback),
    )?;
    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![event_for(added)],
        Some(&callback),
    )?;

    let pending = repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "notes/external.md");
    assert_eq!(pending[0].status, ChangeStatus::Added);

    let messages = messages.lock().expect("messages lock");
    assert_eq!(
        messages.len(),
        1,
        "duplicate watcher signal should not emit a second refresh"
    );
    match &messages[0] {
        crate::protocol::ServerMessage::FsChangeDetected {
            path, change_type, ..
        } => {
            assert_eq!(path, "notes/external.md");
            assert_eq!(change_type, "added");
        }
        other => panic!("unexpected message: {other:?}"),
    }
    Ok(())
}

#[test]
fn dispatch_batch_suppresses_duplicate_rename_pair_messages() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    let doc_id = commit_doc(&repo, &sync, &repo_name, "notes/a.md", "base")?;
    let old = repo_root.join("notes").join("a.md");
    let new = repo_root.join("notes").join("b.md");
    std::fs::rename(&old, &new)?;

    let messages = Arc::new(Mutex::new(Vec::new()));
    let callback_messages = messages.clone();
    let callback: super::WatcherCallback = Arc::new(move |msg| {
        callback_messages.lock().expect("messages lock").push(msg);
    });

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![rename_event(old.clone(), new.clone())],
        Some(&callback),
    )?;
    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![rename_event(old, new)],
        Some(&callback),
    )?;

    let pending = repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert_eq!(pending.len(), 2);
    assert!(pending.iter().any(|entry| {
        entry.path == "notes/a.md"
            && entry.status == ChangeStatus::Deleted
            && entry.doc_id == Some(doc_id)
    }));
    assert!(pending.iter().any(|entry| {
        entry.path == "notes/b.md"
            && entry.status == ChangeStatus::Added
            && entry.renamed_from.as_deref() == Some("notes/a.md")
            && entry.doc_id == Some(doc_id)
    }));

    let messages = messages.lock().expect("messages lock");
    assert_eq!(
        messages.len(),
        2,
        "duplicate rename-pair signal should not emit another delete/add refresh pair"
    );
    assert_fs_message(&messages[0], "notes/a.md", "deleted");
    assert_fs_message(&messages[1], "notes/b.md", "added");
    Ok(())
}

#[test]
fn dispatch_batch_suppresses_duplicate_rename_refresh_from_plain_events() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    let doc_id = commit_doc(&repo, &sync, &repo_name, "notes/a.md", "base")?;
    let old = repo_root.join("notes").join("a.md");
    let new = repo_root.join("notes").join("b.md");
    std::fs::rename(old, &new)?;

    let messages = Arc::new(Mutex::new(Vec::new()));
    let callback_messages = messages.clone();
    let callback: super::WatcherCallback = Arc::new(move |msg| {
        callback_messages.lock().expect("messages lock").push(msg);
    });

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![event_for(new.clone())],
        Some(&callback),
    )?;
    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![event_for(new)],
        Some(&callback),
    )?;

    let pending = repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert_eq!(pending.len(), 2);
    assert!(pending.iter().any(|entry| {
        entry.path == "notes/a.md"
            && entry.status == ChangeStatus::Deleted
            && entry.doc_id == Some(doc_id)
    }));
    assert!(pending.iter().any(|entry| {
        entry.path == "notes/b.md"
            && entry.status == ChangeStatus::Added
            && entry.renamed_from.as_deref() == Some("notes/a.md")
            && entry.doc_id == Some(doc_id)
    }));

    let messages = messages.lock().expect("messages lock");
    assert_eq!(
        messages.len(),
        2,
        "duplicate plain file signals after rename should not emit another refresh pair"
    );
    assert_fs_message(&messages[0], "notes/a.md", "deleted");
    assert_fs_message(&messages[1], "notes/b.md", "added");
    Ok(())
}

#[test]
fn dispatch_batch_fails_closed_on_dir_change_resolution_error() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    let docs = repo_root.join("docs");
    std::fs::create_dir_all(&docs)?;
    repo.run_on_local_repo(&repo_name, |db| {
        let write = db.begin_write()?;
        write
            .open_table(REPO_METADATA)?
            .insert(&0, b"not-bincode".as_slice())?;
        write.commit()?;
        Ok::<_, anyhow::Error>(())
    })?;

    let err = dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![event_for(docs)],
        None,
    )
    .expect_err("dir change resolution must fail closed");

    assert!(err.to_string().contains("Failed to handle dir change"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn dispatch_batch_fails_closed_on_unstatable_dir_event() -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let (_dir, _repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    let blocked = repo_root.join("blocked");
    std::fs::create_dir_all(&blocked)?;
    let original = std::fs::metadata(&blocked)?.permissions();
    let mut blocked_perms = original.clone();
    blocked_perms.set_mode(0o000);
    std::fs::set_permissions(&blocked, blocked_perms)?;

    let result = dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![event_for(blocked.clone())],
        None,
    );
    std::fs::set_permissions(&blocked, original)?;
    let err = result.expect_err("unstatable dir event must fail closed");
    let detail = err.to_string();
    assert!(
        detail.contains("Failed to classify watcher event")
            || detail.contains("Failed to handle dir change")
            || detail.contains("Permission denied"),
        "unexpected error: {detail}"
    );
    Ok(())
}
