//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::{WatcherCallback, WatcherError, filter};
use crate::models::RepoId;
use crate::sync::{SyncManager, pending, pending_rename};
use crate::utils::path::to_forward_slash;
use crate::watcher_ignore::IgnoreRules;
use notify_debouncer_full::{
    DebouncedEvent,
    notify::{
        EventKind,
        event::{ModifyKind, RemoveKind, RenameMode},
    },
};
use std::path::Path;
use std::sync::Arc;

struct DispatchContext<'a> {
    sync: &'a Arc<SyncManager>,
    repo_name: &'a str,
    repo_id: RepoId,
    repo_root: &'a Path,
    ignore_rules: &'a IgnoreRules,
    callback: Option<&'a WatcherCallback>,
}

#[derive(Clone, Copy)]
struct DirectoryEvent {
    allows_refresh: bool,
    removed_candidate: bool,
}

impl DirectoryEvent {
    const RENAME: Self = Self {
        allows_refresh: true,
        removed_candidate: false,
    };

    fn from_event(event: &DebouncedEvent) -> Self {
        Self {
            allows_refresh: filter::allows_directory_refresh(&event.kind),
            removed_candidate: is_removed_dir_candidate(event),
        }
    }
}

#[derive(Default)]
struct BatchDispatchState {
    removed_dir_rescan_done: bool,
}

pub(crate) fn dispatch_batch(
    sync: &Arc<SyncManager>,
    repo_name: &str,
    repo_id: RepoId,
    repo_root: &Path,
    events: Vec<DebouncedEvent>,
    callback: Option<&WatcherCallback>,
) -> Result<(), WatcherError> {
    let ignore_rules = IgnoreRules::load(repo_root);
    let context = DispatchContext {
        sync,
        repo_name,
        repo_id,
        repo_root,
        ignore_rules: &ignore_rules,
        callback,
    };
    let mut state = BatchDispatchState::default();
    for event in events {
        if is_rename(&event) && event.paths.len() >= 2 {
            dispatch_rename(&context, &event.paths)?;
            continue;
        }
        let directory_event = DirectoryEvent::from_event(&event);
        for path in &event.paths {
            dispatch_path(&context, path, directory_event, &mut state)?;
        }
    }
    Ok(())
}

fn ignored_by_rules(rules: &IgnoreRules, root_relative: &str, repo_path: &str) -> bool {
    rules.is_ignored_workspace_path(root_relative, repo_path)
}

fn dispatch_rename(
    context: &DispatchContext<'_>,
    paths: &[std::path::PathBuf],
) -> Result<(), WatcherError> {
    let Some(old_path) = repo_path(context.repo_root, &paths[0]) else {
        return Ok(());
    };
    let Some(new_path) = repo_path(context.repo_root, &paths[1]) else {
        return Ok(());
    };
    let old_root_relative = context
        .sync
        .repo
        .local_repo_workspace_relative(context.repo_name, &old_path);
    let new_root_relative = context
        .sync
        .repo
        .local_repo_workspace_relative(context.repo_name, &new_path);
    let old_ignored = ignored_by_rules(context.ignore_rules, &old_root_relative, &old_path);
    let new_ignored = ignored_by_rules(context.ignore_rules, &new_root_relative, &new_path);
    if old_ignored && new_ignored {
        return Ok(());
    }
    if old_ignored || new_ignored {
        dispatch_rename_as_path_events(context, paths)?;
        return Ok(());
    }
    let old_self_write = context
        .sync
        .should_ignore_fs_event(context.repo_name, &old_path);
    let new_self_write = context
        .sync
        .should_ignore_fs_event(context.repo_name, &new_path);
    if old_self_write && new_self_write {
        return Ok(());
    }
    if old_self_write || new_self_write {
        dispatch_rename_as_path_events(context, paths)?;
        return Ok(());
    }
    if !filter::allows_repo_path(&old_path) || !filter::allows_repo_path(&new_path) {
        dispatch_rename_as_path_events(context, paths)?;
        return Ok(());
    }
    let doc_id = rename_doc_id(context.sync, context.repo_name, &new_path)?.or_else(|| {
        context
            .sync
            .repo
            .resolve_canonical_doc_id_in_local_repo(context.repo_name, &old_path)
            .ok()
            .flatten()
    });
    if let Some(doc_id) = doc_id {
        let changed = pending_rename::upsert_external_rename(
            &context.sync.repo,
            context.repo_name,
            &old_path,
            &new_path,
            doc_id,
        )?;
        if !changed {
            return Ok(());
        }
        if let Some(cb) = context.callback {
            cb(pending::message(
                &context.sync.repo,
                context.repo_name,
                context.repo_id,
                &old_path,
                "deleted",
            )?);
            cb(pending::message(
                &context.sync.repo,
                context.repo_name,
                context.repo_id,
                &new_path,
                "added",
            )?);
        }
    } else {
        dispatch_rename_as_path_events(context, paths)?;
    }
    Ok(())
}

fn dispatch_rename_as_path_events(
    context: &DispatchContext<'_>,
    paths: &[std::path::PathBuf],
) -> Result<(), WatcherError> {
    let mut state = BatchDispatchState::default();
    dispatch_path(context, &paths[0], DirectoryEvent::RENAME, &mut state)?;
    dispatch_path(context, &paths[1], DirectoryEvent::RENAME, &mut state)
}

fn dispatch_path(
    context: &DispatchContext<'_>,
    path: &Path,
    directory_event: DirectoryEvent,
    state: &mut BatchDispatchState,
) -> Result<(), WatcherError> {
    let Some(repo_path) = repo_path(context.repo_root, path) else {
        return Ok(());
    };
    let root_relative = context
        .sync
        .repo
        .local_repo_workspace_relative(context.repo_name, &repo_path);
    if ignored_by_rules(context.ignore_rules, &root_relative, &repo_path) {
        return Ok(());
    }
    if !filter::allows_repo_path(&repo_path) {
        dispatch_dir_change(
            context,
            &root_relative,
            &repo_path,
            path,
            directory_event,
            state,
        )?;
        return Ok(());
    }
    if context
        .sync
        .should_ignore_fs_event(context.repo_name, &repo_path)
    {
        return Ok(());
    }
    for msg in context
        .sync
        .handle_fs_event(context.repo_name, context.repo_id, &repo_path)?
    {
        if let Some(cb) = context.callback {
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
    context: &DispatchContext<'_>,
    root_relative: &str,
    repo_path: &str,
    path: &Path,
    directory_event: DirectoryEvent,
    state: &mut BatchDispatchState,
) -> Result<(), WatcherError> {
    if !directory_event.allows_refresh
        || !filter::allows_repo_dir_path(repo_path)
        || (!directory_event.removed_candidate && !is_directory_event(path, root_relative)?)
    {
        return Ok(());
    }
    let refreshed = if directory_event.removed_candidate {
        if state.removed_dir_rescan_done {
            return Ok(());
        }
        let refreshed = context
            .sync
            .force_dir_refresh(context.repo_name, context.repo_id, repo_path)
            .map(Some);
        if refreshed.is_ok() {
            state.removed_dir_rescan_done = true;
        }
        refreshed
    } else {
        context
            .sync
            .handle_dir_change(context.repo_name, context.repo_id, repo_path)
    }
    .map_err(|err| {
        WatcherError::from(anyhow::anyhow!(
            "Failed to handle dir change for {root_relative}: {err}"
        ))
    })?;
    if let Some((refreshed_repo_id, refreshed_path)) = refreshed
        && let Some(cb) = context.callback
    {
        cb(pending::dir_changed_message(
            refreshed_repo_id,
            &refreshed_path,
        ));
    }
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

fn is_removed_dir_candidate(event: &DebouncedEvent) -> bool {
    matches!(
        event.kind,
        EventKind::Remove(kind) if !matches!(kind, RemoveKind::File)
    )
}
