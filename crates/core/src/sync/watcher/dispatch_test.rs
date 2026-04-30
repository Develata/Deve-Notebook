use super::dispatch::dispatch_batch;
use crate::ledger::REPO_METADATA;
use crate::ledger::RepoManager;
use crate::models::{LedgerEntry, PeerId};
use crate::source_control::ChangeStatus;
use crate::sync::SyncManager;
use notify_debouncer_full::{
    DebouncedEvent,
    notify::{
        Event, EventKind,
        event::{ModifyKind, RenameMode},
    },
};
use std::sync::{Arc, Mutex};
use std::time::Instant;

type SyncFixture = (
    tempfile::TempDir,
    Arc<RepoManager>,
    Arc<SyncManager>,
    String,
    crate::models::RepoId,
    std::path::PathBuf,
);

fn new_sync() -> anyhow::Result<SyncFixture> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let vault = dir.path().join("vault");
    std::fs::create_dir_all(&vault)?;
    let mut repo = RepoManager::init(&ledger, 10, None, None)?;
    repo.set_vault_root_checked(&vault)?;
    let repo = Arc::new(repo);
    let sync = Arc::new(SyncManager::new_checked(repo.clone(), vault)?);
    let repo_name = repo.local_repo_name().to_string();
    let repo_id = repo
        .get_repo_info_for(None, Some(&repo_name))?
        .expect("repo info")
        .uuid;
    let repo_root = repo.local_repo_workspace_root(&repo_name)?;
    std::fs::create_dir_all(&repo_root)?;
    Ok((dir, repo, sync, repo_name, repo_id, repo_root))
}

#[test]
fn dispatch_batch_ignores_event_paths_outside_repo_root() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    let outside = repo_root.parent().expect("vault root").join("outside.md");

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        vec![DebouncedEvent::new(
            Event {
                kind: EventKind::Any,
                paths: vec![outside],
                attrs: Default::default(),
            },
            Instant::now(),
        )],
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
        vec![DebouncedEvent::new(
            Event {
                kind: EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
                paths: vec![inside, outside],
                attrs: Default::default(),
            },
            Instant::now(),
        )],
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

fn event_for(path: std::path::PathBuf) -> DebouncedEvent {
    DebouncedEvent::new(
        Event {
            kind: EventKind::Any,
            paths: vec![path],
            attrs: Default::default(),
        },
        Instant::now(),
    )
}

fn rename_event(old: std::path::PathBuf, new: std::path::PathBuf) -> DebouncedEvent {
    DebouncedEvent::new(
        Event {
            kind: EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
            paths: vec![old, new],
            attrs: Default::default(),
        },
        Instant::now(),
    )
}

fn commit_doc(
    repo: &Arc<RepoManager>,
    sync: &Arc<SyncManager>,
    repo_name: &str,
    path: &str,
    content: &str,
) -> anyhow::Result<crate::models::DocId> {
    let (doc_id, _) = repo.apply_file_structure_in_local_repo(repo_name, path, None, "test")?;
    repo.append_generated_op_in_local_repo(repo_name, doc_id, PeerId::new("local"), |seq| {
        LedgerEntry::new_content(
            doc_id,
            crate::models::Op::Insert {
                pos: 0,
                content: content.into(),
            },
            1,
            PeerId::new("local"),
            seq,
            None,
            None,
        )
    })?;
    sync.persist_doc_in_local_repo(repo_name, doc_id)?;
    Ok(doc_id)
}

fn assert_fs_message(message: &crate::protocol::ServerMessage, path: &str, change_type: &str) {
    match message {
        crate::protocol::ServerMessage::FsChangeDetected {
            path: actual_path,
            change_type: actual_change_type,
            ..
        } => {
            assert_eq!(actual_path, path);
            assert_eq!(actual_change_type, change_type);
        }
        other => panic!("unexpected message: {other:?}"),
    }
}
