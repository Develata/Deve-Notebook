//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::capture::{CaptureInput, CaptureReceiver, bounded_capture};
use super::{
    BackendSignal, FsEventHint, FsEventPath, FsWatcherBackend, MAX_HINT_PATH_BYTES,
    MAX_HINTS_PER_BATCH, ReconcileToken,
};
use crate::sync::watcher::WatcherError;
use crate::utils::path::to_forward_slash;
use notify_debouncer_full::{
    DebounceEventResult, DebouncedEvent, Debouncer, RecommendedCache, new_debouncer,
    notify::{
        EventKind, RecommendedWatcher, RecursiveMode,
        event::{CreateKind, MetadataKind, ModifyKind, RemoveKind, RenameMode},
    },
};
use std::path::Path;
use std::time::Duration;

pub(crate) fn start(
    repo_root: &Path,
    debounce: Duration,
) -> Result<Box<dyn FsWatcherBackend>, WatcherError> {
    let root = std::fs::canonicalize(repo_root).map_err(anyhow::Error::from)?;
    let (capture_tx, capture_rx) = bounded_capture();
    let root_clone = root.clone();
    let mut debouncer = new_debouncer(debounce, None, move |result: DebounceEventResult| {
        capture_tx.submit(normalize(&root_clone, result));
    })
    .map_err(|err| WatcherError::WatcherInitFailed(err.to_string()))?;
    debouncer
        .watch(&root, RecursiveMode::Recursive)
        .map_err(anyhow::Error::from)?;
    Ok(Box::new(NotifyBackend {
        capture: capture_rx,
        debouncer: Some(debouncer),
    }))
}

struct NotifyBackend {
    capture: CaptureReceiver,
    debouncer: Option<Debouncer<RecommendedWatcher, RecommendedCache>>,
}

impl FsWatcherBackend for NotifyBackend {
    fn recv(&self, timeout: Duration) -> Result<Option<BackendSignal>, WatcherError> {
        self.capture.recv(timeout)
    }

    fn complete_reconcile(&self, token: ReconcileToken) -> bool {
        self.capture.complete_reconcile(token)
    }

    fn stop(&mut self) -> Result<(), WatcherError> {
        if let Some(debouncer) = self.debouncer.take() {
            debouncer.stop();
        }
        Ok(())
    }
}

fn normalize(repo_root: &Path, result: DebounceEventResult) -> CaptureInput {
    let events = match result {
        Ok(events) => events,
        Err(_) => return CaptureInput::Reconcile,
    };
    if events.iter().any(|event| event.need_rescan()) {
        return CaptureInput::Reconcile;
    }

    let mut hints = Vec::new();
    let mut hint_path_bytes = 0usize;
    for event in events {
        match normalize_event(repo_root, &event) {
            CaptureInput::Hints(event_hints) => {
                let Some(next_len) = hints.len().checked_add(event_hints.len()) else {
                    return CaptureInput::Reconcile;
                };
                let event_path_bytes = event_hints.iter().try_fold(0usize, |total, hint| {
                    total.checked_add(hint.path_payload_bytes())
                });
                let Some(next_path_bytes) =
                    event_path_bytes.and_then(|bytes| hint_path_bytes.checked_add(bytes))
                else {
                    return CaptureInput::Reconcile;
                };
                if next_len > MAX_HINTS_PER_BATCH || next_path_bytes > MAX_HINT_PATH_BYTES {
                    return CaptureInput::Reconcile;
                }
                hints.extend(event_hints);
                hint_path_bytes = next_path_bytes;
            }
            CaptureInput::Reconcile => return CaptureInput::Reconcile,
            CaptureInput::Ignore => {}
        }
    }
    if hints.is_empty() {
        CaptureInput::Ignore
    } else {
        CaptureInput::Hints(hints)
    }
}

fn normalize_event(repo_root: &Path, event: &DebouncedEvent) -> CaptureInput {
    if matches!(
        event.kind,
        EventKind::Access(_) | EventKind::Modify(ModifyKind::Metadata(MetadataKind::AccessTime))
    ) {
        return CaptureInput::Ignore;
    }
    let paths = match scope_paths(repo_root, &event.paths) {
        Ok(paths) => paths,
        Err(()) => return CaptureInput::Reconcile,
    };
    if paths
        .inside
        .iter()
        .any(|path| is_deveignore_path(path.as_str()))
    {
        return CaptureInput::Reconcile;
    }
    if event.paths.is_empty() {
        return CaptureInput::Reconcile;
    }

    match &event.kind {
        EventKind::Any | EventKind::Other => CaptureInput::Reconcile,
        EventKind::Access(_) => CaptureInput::Ignore,
        EventKind::Create(CreateKind::File | CreateKind::Folder) => changed_hints(paths.inside),
        EventKind::Create(CreateKind::Any | CreateKind::Other) => CaptureInput::Reconcile,
        EventKind::Modify(ModifyKind::Data(_)) => changed_hints(paths.inside),
        EventKind::Modify(ModifyKind::Metadata(MetadataKind::AccessTime)) => CaptureInput::Ignore,
        EventKind::Modify(ModifyKind::Metadata(
            MetadataKind::WriteTime
            | MetadataKind::Permissions
            | MetadataKind::Ownership
            | MetadataKind::Extended,
        )) => changed_hints(paths.inside),
        EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any | MetadataKind::Other))
        | EventKind::Modify(ModifyKind::Any | ModifyKind::Other) => CaptureInput::Reconcile,
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
            normalize_paired_rename(event, paths)
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
            map_hints(paths.inside, FsEventHint::removed_unknown)
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => changed_hints(paths.inside),
        EventKind::Modify(ModifyKind::Name(RenameMode::Any | RenameMode::Other)) => {
            CaptureInput::Reconcile
        }
        EventKind::Remove(RemoveKind::File) => map_hints(paths.inside, FsEventHint::removed_file),
        EventKind::Remove(RemoveKind::Folder) => {
            map_hints(paths.inside, FsEventHint::removed_directory)
        }
        EventKind::Remove(RemoveKind::Any | RemoveKind::Other) => CaptureInput::Reconcile,
    }
}

fn is_deveignore_path(path: &str) -> bool {
    path.eq_ignore_ascii_case(".deveignore")
}

fn changed_hints(paths: Vec<FsEventPath>) -> CaptureInput {
    map_hints(paths, FsEventHint::changed)
}

fn map_hints(paths: Vec<FsEventPath>, constructor: fn(FsEventPath) -> FsEventHint) -> CaptureInput {
    if paths.is_empty() {
        CaptureInput::Ignore
    } else {
        CaptureInput::Hints(paths.into_iter().map(constructor).collect())
    }
}

fn normalize_paired_rename(event: &DebouncedEvent, mut paths: ScopedPaths) -> CaptureInput {
    if event.paths.len() != 2 || paths.outside_count != 0 || paths.inside.len() != 2 {
        return CaptureInput::Reconcile;
    }
    let new_path = paths.inside.pop().expect("paired rename target");
    let old_path = paths.inside.pop().expect("paired rename source");
    CaptureInput::Hints(vec![FsEventHint::rename(old_path, new_path)])
}

struct ScopedPaths {
    inside: Vec<FsEventPath>,
    outside_count: usize,
}

fn scope_paths(repo_root: &Path, paths: &[std::path::PathBuf]) -> Result<ScopedPaths, ()> {
    if paths.len() > MAX_HINTS_PER_BATCH {
        return Err(());
    }
    let mut inside = Vec::with_capacity(paths.len());
    let mut outside_count = 0;
    let mut path_bytes = 0usize;
    for path in paths {
        let relative = match path.strip_prefix(repo_root) {
            Ok(relative) => relative,
            Err(_) => {
                outside_count += 1;
                continue;
            }
        };
        let relative = relative.to_str().ok_or(())?;
        path_bytes = path_bytes.checked_add(relative.len()).ok_or(())?;
        if path_bytes > MAX_HINT_PATH_BYTES {
            return Err(());
        }
        let normalized = to_forward_slash(relative);
        inside.push(FsEventPath::new(normalized).ok_or(())?);
    }
    Ok(ScopedPaths {
        inside,
        outside_count,
    })
}

#[cfg(test)]
mod tests {
    use super::normalize;
    use crate::sync::watcher::backend::{
        FsEventHintKind, MAX_HINT_PATH_BYTES, MAX_HINTS_PER_BATCH, capture::CaptureInput,
    };
    use notify_debouncer_full::{
        DebouncedEvent,
        notify::{
            Error, Event, EventKind,
            event::{
                AccessKind, AccessMode, CreateKind, DataChange, Flag, ModifyKind, RemoveKind,
                RenameMode,
            },
        },
    };
    use std::path::Path;
    use std::time::Instant;

    fn event(kind: EventKind, paths: Vec<std::path::PathBuf>) -> DebouncedEvent {
        DebouncedEvent::new(
            Event {
                kind,
                paths,
                attrs: Default::default(),
            },
            Instant::now(),
        )
    }

    #[test]
    fn notify_backend_error_requests_rescan() {
        assert_eq!(
            normalize(Path::new("/repo"), Err(vec![Error::generic("overflow")])),
            CaptureInput::Reconcile
        );
    }

    #[test]
    fn notify_rescan_flag_requests_rescan() {
        let event = Event::new(EventKind::Other).set_flag(Flag::Rescan);
        assert_eq!(
            normalize(
                Path::new("/repo"),
                Ok(vec![DebouncedEvent::new(event, Instant::now())])
            ),
            CaptureInput::Reconcile
        );
    }

    #[test]
    fn notify_paths_are_repo_relative_utf8_domain_values() {
        let dir = tempfile::tempdir().expect("tempdir");
        let note = dir.path().join("notes").join("a.md");
        let input = normalize(
            dir.path(),
            Ok(vec![event(EventKind::Create(CreateKind::File), vec![note])]),
        );
        match input {
            CaptureInput::Hints(hints) => {
                assert_eq!(hints.len(), 1);
                assert_eq!(hints[0].kind(), FsEventHintKind::Changed);
                assert_eq!(hints[0].paths()[0].as_str(), "notes/a.md");
            }
            other => panic!("expected normalized hint: {other:?}"),
        }
    }

    #[test]
    fn notify_removed_directory_path_is_kept_for_rescan() {
        let dir = tempfile::tempdir().expect("tempdir");
        let removed_dir = dir.path().join("notes");
        let input = normalize(
            dir.path(),
            Ok(vec![event(
                EventKind::Remove(RemoveKind::Folder),
                vec![removed_dir],
            )]),
        );
        match input {
            CaptureInput::Hints(hints) => {
                assert_eq!(hints.len(), 1);
                assert_eq!(hints[0].kind(), FsEventHintKind::RemovedDirectory);
            }
            other => panic!("remove directory should remain incremental: {other:?}"),
        }
    }

    #[test]
    fn notify_removed_non_markdown_file_stays_a_domain_hint() {
        let dir = tempfile::tempdir().expect("tempdir");
        let removed_file = dir.path().join("scratch.tmp");
        let input = normalize(
            dir.path(),
            Ok(vec![event(
                EventKind::Remove(RemoveKind::File),
                vec![removed_file],
            )]),
        );
        match input {
            CaptureInput::Hints(hints) => {
                assert_eq!(hints.len(), 1);
                assert_eq!(hints[0].kind(), FsEventHintKind::RemovedFile);
                assert_eq!(hints[0].paths()[0].as_str(), "scratch.tmp");
            }
            other => panic!("semantic filtering belongs after the adapter: {other:?}"),
        }
    }

    #[test]
    fn watcher_ignore_change_sets_reconcile_before_filter() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ignore = dir.path().join(".deveignore");
        assert_eq!(
            normalize(
                dir.path(),
                Ok(vec![event(
                    EventKind::Modify(ModifyKind::Data(DataChange::Content)),
                    vec![ignore]
                )])
            ),
            CaptureInput::Reconcile
        );
    }

    #[test]
    fn watcher_ignore_access_is_ignored_before_dirtying_capture() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ignore = dir.path().join(".DEVEIGNORE");
        assert_eq!(
            normalize(
                dir.path(),
                Ok(vec![event(
                    EventKind::Access(AccessKind::Open(AccessMode::Any)),
                    vec![ignore]
                )])
            ),
            CaptureInput::Ignore
        );
    }

    #[test]
    fn watcher_cross_root_rename_sets_reconcile() {
        let dir = tempfile::tempdir().expect("tempdir");
        let inside = dir.path().join("note.md");
        let outside = dir.path().parent().expect("parent").join("outside.md");
        assert_eq!(
            normalize(
                dir.path(),
                Ok(vec![event(
                    EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
                    vec![inside, outside]
                )])
            ),
            CaptureInput::Reconcile
        );
    }

    #[test]
    fn notify_non_rename_outside_root_is_ignored() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outside = dir.path().parent().expect("parent").join("outside.md");
        assert_eq!(
            normalize(
                dir.path(),
                Ok(vec![event(
                    EventKind::Create(CreateKind::File),
                    vec![outside]
                )])
            ),
            CaptureInput::Ignore
        );
    }

    #[test]
    fn notify_unknown_and_zero_path_mutations_request_reconcile() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(
            normalize(
                dir.path(),
                Ok(vec![event(EventKind::Any, vec![dir.path().join("a.md")])])
            ),
            CaptureInput::Reconcile
        );
        assert_eq!(
            normalize(
                dir.path(),
                Ok(vec![event(EventKind::Create(CreateKind::File), Vec::new())])
            ),
            CaptureInput::Reconcile
        );
    }

    #[test]
    fn notify_access_event_is_filtered_before_capture() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("notes");
        assert_eq!(
            normalize(
                dir.path(),
                Ok(vec![event(
                    EventKind::Access(AccessKind::Open(AccessMode::Any)),
                    vec![path]
                )])
            ),
            CaptureInput::Ignore
        );
    }

    #[test]
    fn notify_oversized_event_path_count_requests_reconcile_before_copying() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = (0..=MAX_HINTS_PER_BATCH)
            .map(|index| dir.path().join(format!("note-{index}.md")))
            .collect();
        assert_eq!(
            normalize(
                dir.path(),
                Ok(vec![event(EventKind::Create(CreateKind::File), paths)])
            ),
            CaptureInput::Reconcile
        );
    }

    #[test]
    fn notify_oversized_path_payload_requests_reconcile_before_copying() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("a".repeat(MAX_HINT_PATH_BYTES + 1));
        assert_eq!(
            normalize(
                dir.path(),
                Ok(vec![event(EventKind::Create(CreateKind::File), vec![path])])
            ),
            CaptureInput::Reconcile
        );
    }

    #[test]
    fn notify_aggregate_hint_limit_requests_reconcile_without_partial_batch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let events = (0..=MAX_HINTS_PER_BATCH)
            .map(|index| {
                event(
                    EventKind::Create(CreateKind::File),
                    vec![dir.path().join(format!("note-{index}.md"))],
                )
            })
            .collect();
        assert_eq!(normalize(dir.path(), Ok(events)), CaptureInput::Reconcile);
    }
}
