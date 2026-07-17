use super::dispatch::dispatch_batch;
use super::dispatch_test_support::{
    assert_fs_message, commit_doc, event_for, ledger_op_count, new_sync,
};
use crate::source_control::ChangeStatus;
use std::sync::{Arc, Mutex};

#[test]
fn dispatch_batch_collapses_modified_burst_by_content_hash() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id, repo_root) = new_sync()?;
    let doc_id = commit_doc(&repo, &sync, &repo_name, "notes/debounce.md", "base\n")?;
    let file = repo_root.join("notes").join("debounce.md");
    let before_ledger_ops = ledger_op_count(&repo, &repo_name)?;

    for _ in 0..5 {
        let mut content = std::fs::read_to_string(&file)?;
        content.push('x');
        std::fs::write(&file, content)?;
    }
    let final_content = std::fs::read_to_string(&file)?;
    let final_hash = crate::source_control::pending_fs::content_hash(&final_content);

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
        (0..5)
            .map(|_| event_for(&repo_root, file.clone()))
            .collect::<Vec<_>>(),
        Some(&callback),
    )?;

    let pending = repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "notes/debounce.md");
    assert_eq!(pending[0].status, ChangeStatus::Modified);
    assert_eq!(pending[0].doc_id, Some(doc_id));
    let pending_entry = repo
        .run_on_local_repo(&repo_name, |db| {
            crate::source_control::pending_fs::get(db, "notes/debounce.md")
        })?
        .expect("pending fs entry");
    assert_eq!(pending_entry.content_hash, final_hash);
    let detected_at = pending_entry.detected_at;
    assert_eq!(
        ledger_op_count(&repo, &repo_name)?,
        before_ledger_ops,
        "watcher burst must not append ledger ops before stage/commit"
    );

    let locked_messages = messages.lock().expect("messages lock");
    assert_eq!(
        locked_messages.len(),
        1,
        "same final content hash should produce one pending refresh"
    );
    assert_fs_message(&locked_messages[0], "notes/debounce.md", "modified");
    drop(locked_messages);

    dispatch_batch(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        (0..5)
            .map(|_| event_for(&repo_root, file.clone()))
            .collect::<Vec<_>>(),
        Some(&callback),
    )?;

    let pending_after_repeat = repo.list_pending_fs_in_local_repo(&repo_name)?;
    assert_eq!(pending_after_repeat.len(), 1);
    let pending_entry_after_repeat = repo
        .run_on_local_repo(&repo_name, |db| {
            crate::source_control::pending_fs::get(db, "notes/debounce.md")
        })?
        .expect("pending fs entry after repeat");
    assert_eq!(pending_entry_after_repeat.content_hash, final_hash);
    assert_eq!(
        pending_entry_after_repeat.detected_at, detected_at,
        "duplicate content hash must preserve pending row timestamp"
    );
    assert_eq!(ledger_op_count(&repo, &repo_name)?, before_ledger_ops);
    assert_eq!(
        messages.lock().expect("messages lock").len(),
        1,
        "duplicate burst should not emit another refresh"
    );
    Ok(())
}
