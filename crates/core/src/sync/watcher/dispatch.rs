//! plan_ref:
//!   - 04_storage#watcher-contract

use super::{WatcherCallback, WatcherError, filter};
use crate::models::RepoId;
use crate::sync::{SyncManager, pending, pending_rename};
use crate::utils::path::to_forward_slash;
use notify_debouncer_full::{
    DebouncedEvent,
    notify::event::{ModifyKind, RenameMode},
};
use std::path::Path;
use std::sync::Arc;

pub(crate) fn dispatch_batch(
    sync: &Arc<SyncManager>,
    repo_name: &str,
    repo_id: RepoId,
    repo_root: &Path,
    events: Vec<DebouncedEvent>,
    callback: Option<&WatcherCallback>,
) -> Result<(), WatcherError> {
    for event in events {
        if is_rename(&event) && event.paths.len() >= 2 {
            dispatch_rename(sync, repo_name, repo_id, repo_root, &event.paths, callback)?;
            continue;
        }
        for path in &event.paths {
            let Some(repo_path) = repo_path(repo_root, path)? else {
                continue;
            };
            let root_relative = sync
                .repo
                .local_repo_workspace_relative(repo_name, &repo_path);
            if sync.should_ignore_fs_event(&root_relative) {
                continue;
            }
            for msg in sync.handle_fs_event(&root_relative)? {
                if let Some(cb) = callback {
                    cb(msg);
                }
            }
        }
    }
    Ok(())
}

fn dispatch_rename(
    sync: &Arc<SyncManager>,
    repo_name: &str,
    repo_id: RepoId,
    repo_root: &Path,
    paths: &[std::path::PathBuf],
    callback: Option<&WatcherCallback>,
) -> Result<(), WatcherError> {
    let Some(old_path) = repo_path(repo_root, &paths[0])? else {
        return Ok(());
    };
    let Some(new_path) = repo_path(repo_root, &paths[1])? else {
        return Ok(());
    };
    if !filter::allows_repo_path(&old_path) || !filter::allows_repo_path(&new_path) {
        return Ok(());
    }
    let doc_id = rename_doc_id(sync, repo_name, &new_path)?.or_else(|| {
        sync.repo
            .resolve_canonical_doc_id_in_local_repo(repo_name, &old_path)
            .ok()
            .flatten()
    });
    if let Some(doc_id) = doc_id {
        pending_rename::upsert_external_rename(
            &sync.repo, repo_name, &old_path, &new_path, doc_id,
        )?;
        if let Some(cb) = callback {
            cb(pending::message(
                &sync.repo, repo_name, repo_id, &old_path, "deleted",
            )?);
            cb(pending::message(
                &sync.repo, repo_name, repo_id, &new_path, "added",
            )?);
        }
    }
    Ok(())
}

fn repo_path(repo_root: &Path, path: &Path) -> Result<Option<String>, WatcherError> {
    let rel = path
        .strip_prefix(repo_root)
        .map_err(|_| WatcherError::PathEscaped(path.into()))?;
    let path = to_forward_slash(&rel.to_string_lossy());
    Ok(filter::allows_repo_path(&path).then_some(path))
}

fn rename_doc_id(
    sync: &Arc<SyncManager>,
    repo_name: &str,
    repo_path: &str,
) -> Result<Option<crate::models::DocId>, WatcherError> {
    let root_rel = sync
        .repo
        .local_repo_workspace_relative(repo_name, repo_path);
    let Some(inode) = sync.vfs.get_inode(&root_rel)? else {
        return Ok(None);
    };
    Ok(sync
        .repo
        .get_docid_by_inode_in_local_repo(repo_name, &inode)?)
}

fn is_rename(event: &DebouncedEvent) -> bool {
    matches!(
        event.kind,
        notify_debouncer_full::notify::EventKind::Modify(ModifyKind::Name(RenameMode::Both))
    )
}
