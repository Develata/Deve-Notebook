//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::super::capture::bounded_capture;
use super::{normalize, submit_callback};
use crate::sync::watcher::backend::{
    FsEventHintKind, MAX_HINT_PATH_BYTES, MAX_HINTS_PER_BATCH, capture::CaptureInput,
};
use crate::sync::watcher::{WatcherFailureKind, WatcherFailurePhase};
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
fn watcher_capture_first_startup_callback_panic_is_typed_terminal() {
    let (sender, receiver) = bounded_capture(31);
    let token = receiver.begin_startup_scan().expect("startup pass");
    submit_callback(&sender, || panic!("injected notify callback panic"));
    drop(sender);

    let failure = receiver
        .complete_startup_scan(token)
        .expect_err("callback panic must terminate startup");
    assert_eq!(failure.phase, WatcherFailurePhase::Receive);
    assert_eq!(failure.kind, WatcherFailureKind::Panic);
    assert!(failure.primary.contains("injected notify callback panic"));
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
