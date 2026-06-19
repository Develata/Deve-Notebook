//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::{BackendBatch, FsWatcherBackend};
use crate::sync::watcher::{WatcherError, filter};
use crate::utils::path::to_forward_slash;
use notify_debouncer_full::{
    DebounceEventResult, Debouncer, RecommendedCache, new_debouncer,
    notify::{EventKind, RecommendedWatcher, RecursiveMode, event::RemoveKind},
};
use std::path::Path;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::Duration;

pub(crate) fn start(
    repo_root: &Path,
    debounce: Duration,
) -> Result<Box<dyn FsWatcherBackend>, WatcherError> {
    let root = std::fs::canonicalize(repo_root).map_err(anyhow::Error::from)?;
    let (tx, rx) = channel();
    let root_clone = root.clone();
    let mut debouncer = new_debouncer(debounce, None, move |result: DebounceEventResult| {
        let _ = tx.send(normalize(&root_clone, result));
    })
    .map_err(|err| WatcherError::WatcherInitFailed(err.to_string()))?;
    debouncer
        .watch(&root, RecursiveMode::Recursive)
        .map_err(anyhow::Error::from)?;
    Ok(Box::new(NotifyBackend {
        rx,
        debouncer: Some(debouncer),
    }))
}

struct NotifyBackend {
    rx: Receiver<Result<BackendBatch, WatcherError>>,
    debouncer: Option<Debouncer<RecommendedWatcher, RecommendedCache>>,
}

impl FsWatcherBackend for NotifyBackend {
    fn recv(&self, timeout: Duration) -> Result<Option<BackendBatch>, WatcherError> {
        match self.rx.recv_timeout(timeout) {
            Ok(batch) => batch.map(Some),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(WatcherError::WatcherInitFailed(
                "backend channel closed".into(),
            )),
        }
    }

    fn stop(&mut self) -> Result<(), WatcherError> {
        self.debouncer.take();
        Ok(())
    }
}

fn normalize(repo_root: &Path, result: DebounceEventResult) -> Result<BackendBatch, WatcherError> {
    let events = match result {
        Ok(events) => events,
        Err(_) => return Ok(BackendBatch::Rescan),
    };
    if events.iter().any(|event| event.need_rescan()) {
        return Ok(BackendBatch::Rescan);
    }
    let events: Vec<_> = events
        .into_iter()
        .filter(|event| keep_event(repo_root, event))
        .collect();
    Ok(BackendBatch::Events(events))
}

fn keep_event(repo_root: &Path, event: &notify_debouncer_full::DebouncedEvent) -> bool {
    let keep_removed_dir_candidate = matches!(
        &event.kind,
        EventKind::Remove(kind) if !matches!(kind, RemoveKind::File)
    );
    event.paths.iter().any(|path| {
        let Ok(rel) = path.strip_prefix(repo_root) else {
            return false;
        };
        let rel = to_forward_slash(&rel.to_string_lossy());
        filter::allows_repo_path(&rel)
            || (filter::allows_repo_dir_path(&rel) && (path.is_dir() || keep_removed_dir_candidate))
    })
}

#[cfg(test)]
mod tests {
    use super::{BackendBatch, normalize};
    use notify_debouncer_full::{
        DebouncedEvent,
        notify::{
            Error, Event, EventKind,
            event::{Flag, RemoveKind},
        },
    };
    use std::path::Path;
    use std::time::Instant;

    #[test]
    fn notify_backend_error_requests_rescan() {
        let batch =
            normalize(Path::new("/repo"), Err(vec![Error::generic("overflow")])).expect("batch");

        assert!(matches!(batch, BackendBatch::Rescan));
    }

    #[test]
    fn notify_rescan_flag_requests_rescan() {
        let event = Event::new(EventKind::Other).set_flag(Flag::Rescan);
        let batch = normalize(
            Path::new("/repo"),
            Ok(vec![DebouncedEvent::new(event, Instant::now())]),
        )
        .expect("batch");

        assert!(matches!(batch, BackendBatch::Rescan));
    }

    #[test]
    fn notify_removed_directory_path_is_kept_for_rescan() {
        let dir = tempfile::tempdir().expect("tempdir");
        let removed_dir = dir.path().join("notes");
        let event = Event::new(EventKind::Remove(RemoveKind::Folder)).add_path(removed_dir);
        let batch = normalize(
            dir.path(),
            Ok(vec![DebouncedEvent::new(event, Instant::now())]),
        )
        .expect("batch");

        match batch {
            BackendBatch::Events(events) => assert_eq!(events.len(), 1),
            BackendBatch::Rescan => panic!("remove dir should stay an incremental dir scan event"),
        }
    }

    #[test]
    fn notify_removed_non_markdown_file_path_is_filtered() {
        let dir = tempfile::tempdir().expect("tempdir");
        let removed_file = dir.path().join("scratch.tmp");
        let event = Event::new(EventKind::Remove(RemoveKind::File)).add_path(removed_file);
        let batch = normalize(
            dir.path(),
            Ok(vec![DebouncedEvent::new(event, Instant::now())]),
        )
        .expect("batch");

        match batch {
            BackendBatch::Events(events) => assert!(events.is_empty()),
            BackendBatch::Rescan => panic!("non-markdown file remove should not request rescan"),
        }
    }
}
