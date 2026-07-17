//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::capture::{CaptureInput, CaptureReceiver, CaptureSender, bounded_capture};
use super::{
    BackendSignal, FsEventHint, FsEventPath, FsWatcherBackend, MAX_HINT_PATH_BYTES,
    MAX_HINTS_PER_BATCH, ReconcileToken, StartupHandoff, StartupScanToken,
};
use crate::sync::watcher::{WatcherError, WatcherFailure, WatcherFailureKind, WatcherFailurePhase};
use crate::utils::path::to_forward_slash;
use notify_debouncer_full::{
    DebounceEventResult, DebouncedEvent, Debouncer, RecommendedCache, new_debouncer,
    notify::{
        EventKind, RecommendedWatcher, RecursiveMode,
        event::{CreateKind, MetadataKind, ModifyKind, RemoveKind, RenameMode},
    },
};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::time::Duration;

pub(crate) fn start(
    repo_root: &Path,
    debounce: Duration,
    generation: u64,
) -> Result<Box<dyn FsWatcherBackend>, WatcherFailure> {
    let root = repo_root.to_path_buf();
    let (capture_tx, capture_rx) = bounded_capture(generation);
    let root_clone = root.clone();
    let mut debouncer = catch_unwind(AssertUnwindSafe(|| {
        new_debouncer(debounce, None, move |result: DebounceEventResult| {
            submit_callback(&capture_tx, || normalize(&root_clone, result));
        })
    }))
    .map_err(|panic| attach_panic("create notify debouncer", panic))?
    .map_err(|error| attach_failure(error.to_string()))?;
    let watch_failure = match catch_unwind(AssertUnwindSafe(|| {
        debouncer.watch(&root, RecursiveMode::Recursive)
    })) {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(attach_failure(error.to_string())),
        Err(panic) => Some(attach_panic("attach recursive watch", panic)),
    };
    if let Some(mut failure) = watch_failure {
        if let Err(panic) = catch_unwind(AssertUnwindSafe(|| debouncer.stop())) {
            failure.cleanup.push(super::super::panic_message(panic));
        }
        return Err(failure);
    }
    Ok(Box::new(NotifyBackend {
        capture: capture_rx,
        debouncer: Some(debouncer),
    }))
}

fn submit_callback(normalized_capture: &CaptureSender, normalize: impl FnOnce() -> CaptureInput) {
    if let Err(panic) = catch_unwind(AssertUnwindSafe(|| {
        normalized_capture.submit(normalize());
    })) {
        normalized_capture.terminate(WatcherFailure::new(
            WatcherFailurePhase::Receive,
            WatcherFailureKind::Panic,
            super::super::panic_message(panic),
        ));
    }
}

struct NotifyBackend {
    capture: CaptureReceiver,
    debouncer: Option<Debouncer<RecommendedWatcher, RecommendedCache>>,
}

impl Drop for NotifyBackend {
    fn drop(&mut self) {
        let Some(debouncer) = self.debouncer.take() else {
            return;
        };
        if let Err(panic) = catch_unwind(AssertUnwindSafe(|| debouncer.stop())) {
            tracing::error!(
                panic = %super::super::panic_message(panic),
                "best-effort notify backend Drop cleanup panicked"
            );
        }
    }
}

impl FsWatcherBackend for NotifyBackend {
    fn begin_startup_scan(&self) -> Result<StartupScanToken, WatcherFailure> {
        self.capture.begin_startup_scan()
    }

    fn complete_startup_scan(
        &self,
        token: StartupScanToken,
    ) -> Result<StartupHandoff, WatcherFailure> {
        self.capture.complete_startup_scan(token)
    }

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

fn attach_failure(primary: impl Into<String>) -> WatcherFailure {
    WatcherFailure::new(
        WatcherFailurePhase::Attach,
        WatcherFailureKind::Backend,
        primary,
    )
}

fn attach_panic(context: &str, panic: Box<dyn std::any::Any + Send>) -> WatcherFailure {
    WatcherFailure::new(
        WatcherFailurePhase::Attach,
        WatcherFailureKind::Panic,
        format!("{context}: {}", super::super::panic_message(panic)),
    )
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
#[path = "notify_impl_tests.rs"]
mod tests;
