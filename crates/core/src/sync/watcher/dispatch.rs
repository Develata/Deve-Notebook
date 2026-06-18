//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::{WatcherCallback, WatcherError, filter};
use crate::models::RepoId;
use crate::sync::{SyncManager, pending, pending_rename};
use crate::utils::path::to_forward_slash;
use crate::watcher_ignore::IgnoreRules;
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
    let ignore_rules = Some(IgnoreRules::load(repo_root));
    for event in events {
        if is_rename(&event) && event.paths.len() >= 2 {
            dispatch_rename(sync, repo_name, repo_id, repo_root, &event.paths, callback)?;
            continue;
        }
        for path in &event.paths {
            dispatch_path(
                sync,
                repo_name,
                repo_id,
                repo_root,
                ignore_rules.as_ref(),
                path,
                callback,
            )?;
        }
    }
    Ok(())
}

fn ignored_by_rules(rules: Option<&IgnoreRules>, root_relative: &str, repo_path: &str) -> bool {
    rules.is_some_and(|rules| rules.is_ignored_workspace_path(root_relative, repo_path))
}

fn dispatch_rename(
    sync: &Arc<SyncManager>,
    repo_name: &str,
    repo_id: RepoId,
    repo_root: &Path,
    paths: &[std::path::PathBuf],
    callback: Option<&WatcherCallback>,
) -> Result<(), WatcherError> {
    let Some(old_path) = repo_path(repo_root, &paths[0]) else {
        return Ok(());
    };
    let Some(new_path) = repo_path(repo_root, &paths[1]) else {
        return Ok(());
    };
    let ignore_rules = Some(IgnoreRules::load(repo_root));
    let old_root_relative = sync
        .repo
        .local_repo_workspace_relative(repo_name, &old_path);
    let new_root_relative = sync
        .repo
        .local_repo_workspace_relative(repo_name, &new_path);
    let old_ignored = ignored_by_rules(ignore_rules.as_ref(), &old_root_relative, &old_path);
    let new_ignored = ignored_by_rules(ignore_rules.as_ref(), &new_root_relative, &new_path);
    if old_ignored && new_ignored {
        return Ok(());
    }
    if old_ignored || new_ignored {
        dispatch_rename_as_path_events(
            sync,
            repo_name,
            repo_id,
            repo_root,
            ignore_rules.as_ref(),
            paths,
            callback,
        )?;
        return Ok(());
    }
    if !filter::allows_repo_path(&old_path) || !filter::allows_repo_path(&new_path) {
        dispatch_rename_as_path_events(
            sync,
            repo_name,
            repo_id,
            repo_root,
            ignore_rules.as_ref(),
            paths,
            callback,
        )?;
        return Ok(());
    }
    let doc_id = rename_doc_id(sync, repo_name, &new_path)?.or_else(|| {
        sync.repo
            .resolve_canonical_doc_id_in_local_repo(repo_name, &old_path)
            .ok()
            .flatten()
    });
    if let Some(doc_id) = doc_id {
        let changed = pending_rename::upsert_external_rename(
            &sync.repo, repo_name, &old_path, &new_path, doc_id,
        )?;
        if !changed {
            return Ok(());
        }
        if let Some(cb) = callback {
            cb(pending::message(
                &sync.repo, repo_name, repo_id, &old_path, "deleted",
            )?);
            cb(pending::message(
                &sync.repo, repo_name, repo_id, &new_path, "added",
            )?);
        }
    } else {
        dispatch_rename_as_path_events(
            sync,
            repo_name,
            repo_id,
            repo_root,
            ignore_rules.as_ref(),
            paths,
            callback,
        )?;
    }
    Ok(())
}

fn dispatch_rename_as_path_events(
    sync: &Arc<SyncManager>,
    repo_name: &str,
    repo_id: RepoId,
    repo_root: &Path,
    ignore_rules: Option<&IgnoreRules>,
    paths: &[std::path::PathBuf],
    callback: Option<&WatcherCallback>,
) -> Result<(), WatcherError> {
    dispatch_path(
        sync,
        repo_name,
        repo_id,
        repo_root,
        ignore_rules,
        &paths[0],
        callback,
    )?;
    dispatch_path(
        sync,
        repo_name,
        repo_id,
        repo_root,
        ignore_rules,
        &paths[1],
        callback,
    )
}

fn dispatch_path(
    sync: &Arc<SyncManager>,
    repo_name: &str,
    repo_id: RepoId,
    repo_root: &Path,
    ignore_rules: Option<&IgnoreRules>,
    path: &Path,
    callback: Option<&WatcherCallback>,
) -> Result<(), WatcherError> {
    let Some(repo_path) = repo_path(repo_root, path) else {
        return Ok(());
    };
    let root_relative = sync
        .repo
        .local_repo_workspace_relative(repo_name, &repo_path);
    if ignored_by_rules(ignore_rules, &root_relative, &repo_path) {
        return Ok(());
    }
    if !filter::allows_repo_path(&repo_path) {
        dispatch_dir_change(sync, repo_name, repo_id, &root_relative, &repo_path, path)?;
        return Ok(());
    }
    if sync.should_ignore_fs_event(repo_name, &repo_path) {
        return Ok(());
    }
    for msg in sync.handle_fs_event(repo_name, repo_id, &repo_path)? {
        if let Some(cb) = callback {
            cb(msg);
        }
    }
    Ok(())
}

fn repo_path(repo_root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(repo_root).ok()?;
    let path = to_forward_slash(&rel.to_string_lossy());
    Some(path)
}

fn dispatch_dir_change(
    sync: &Arc<SyncManager>,
    repo_name: &str,
    repo_id: RepoId,
    root_relative: &str,
    repo_path: &str,
    path: &Path,
) -> Result<(), WatcherError> {
    if !filter::allows_repo_dir_path(repo_path) || !is_directory_event(path, root_relative)? {
        return Ok(());
    }
    sync.handle_dir_change(repo_name, repo_id, repo_path)
        .map_err(|err| {
            WatcherError::from(anyhow::anyhow!(
                "Failed to handle dir change for {root_relative}: {err}"
            ))
        })?;
    Ok(())
}

fn is_directory_event(path: &Path, root_relative: &str) -> Result<bool, WatcherError> {
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(metadata.is_dir()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(anyhow::anyhow!(
            "Failed to classify watcher event for {root_relative}: {err}"
        )
        .into()),
    }
}

fn rename_doc_id(
    sync: &Arc<SyncManager>,
    repo_name: &str,
    repo_path: &str,
) -> Result<Option<crate::models::DocId>, WatcherError> {
    let disk_path = sync.repo.local_repo_workspace_path(repo_name, repo_path)?;
    let Some(inode) = sync.vfs.get_inode_abs(&disk_path)? else {
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
