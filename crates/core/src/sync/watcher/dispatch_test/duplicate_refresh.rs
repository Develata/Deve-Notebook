use super::super::WatcherCallback;
use super::super::dispatch::dispatch_batch;
use super::super::dispatch_test_support::{
    assert_fs_message, commit_doc, event_for, new_sync, rename_event,
};
use crate::source_control::ChangeStatus;
use std::sync::{Arc, Mutex};

#[test]
fn dispatch_batch_suppresses_duplicate_external_added_message() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    let added = repo_root.join("notes").join("external.md");
    std::fs::create_dir_all(added.parent().expect("parent"))?;
    std::fs::write(&added, "external")?;

    let messages = Arc::new(Mutex::new(Vec::new()));
    let callback_messages = messages.clone();
    let callback: WatcherCallback = Arc::new(move |msg| {
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
fn dispatch_batch_does_not_reopen_committed_crlf_file_as_modified() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, _repo_id, repo_root) = new_sync()?;
    let added = repo_root.join("notes").join("crlf.md");
    std::fs::create_dir_all(added.parent().expect("parent"))?;
    std::fs::write(&added, b"# CRLF\r\n\r\nCommitted from disk.\r\n")?;

    crate::sync::scan::scan_local_repo(&repo, &sync.vfs, &repo_name)?;
    repo.stage_pending_in_local_repo(&repo_name, "notes/crlf.md")?;
    repo.commit_staged_in_local_repo(&repo_name, "add crlf file")?;
    assert!(
        repo.list_pending_fs_in_local_repo(&repo_name)?.is_empty(),
        "commit should leave no pending source-control entries"
    );

    crate::sync::scan::scan_local_repo(&repo, &sync.vfs, &repo_name)?;
    assert!(
        repo.list_pending_fs_in_local_repo(&repo_name)?.is_empty(),
        "a post-commit scan must not reopen a CRLF-only drift as modified"
    );
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
    let callback: WatcherCallback = Arc::new(move |msg| {
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
    let callback: WatcherCallback = Arc::new(move |msg| {
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
