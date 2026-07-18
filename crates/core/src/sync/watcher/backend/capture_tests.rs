//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::*;
use crate::sync::watcher::backend::FsEventPath;
use std::sync::{Arc, Barrier, mpsc};

fn hint(index: usize) -> FsEventHint {
    FsEventHint::changed(FsEventPath::new(format!("note-{index}.md")).expect("valid relative path"))
}

fn handoff_running(receiver: &CaptureReceiver) {
    let token = receiver.begin_startup_scan().expect("begin startup scan");
    assert_eq!(
        receiver
            .complete_startup_scan(token)
            .expect("complete startup scan"),
        StartupHandoff::Running
    );
}

#[test]
fn watcher_capture_first_startup_routes_pre_cut_hint_to_dirty_and_post_cut_hint_to_queue() {
    let (sender, receiver) = bounded_capture(7);
    let first = receiver.begin_startup_scan().expect("first startup pass");
    sender.submit(CaptureInput::Hints(vec![hint(1)]));
    assert_eq!(
        receiver
            .complete_startup_scan(first)
            .expect("dirty first handoff"),
        StartupHandoff::Dirty
    );
    assert!(
        receiver.rx.try_recv().is_err(),
        "CaptureOnly hints are level-triggered dirty evidence, not replay payload"
    );

    let second = receiver.begin_startup_scan().expect("second startup pass");
    assert_eq!(
        receiver
            .complete_startup_scan(second)
            .expect("clean second handoff"),
        StartupHandoff::Running
    );
    sender.submit(CaptureInput::Hints(vec![hint(2)]));
    assert!(matches!(
        receiver.recv(Duration::ZERO).expect("running receive"),
        Some(BackendSignal::Hints(batch)) if batch.hints() == [hint(2)]
    ));
}

#[test]
fn watcher_capture_first_startup_has_no_third_hint_handoff_outcome() {
    for generation in 1..=128 {
        let (sender, receiver) = bounded_capture(generation);
        let token = receiver.begin_startup_scan().expect("startup pass");
        let barrier = Arc::new(Barrier::new(2));
        let producer_barrier = barrier.clone();
        let producer = std::thread::spawn(move || {
            producer_barrier.wait();
            sender.submit(CaptureInput::Hints(vec![hint(generation as usize)]));
            sender
        });

        barrier.wait();
        let outcome = receiver
            .complete_startup_scan(token)
            .expect("startup handoff");
        let sender = producer.join().expect("producer thread");
        match outcome {
            StartupHandoff::Dirty => {
                assert!(receiver.rx.try_recv().is_err());
                let retry = receiver.begin_startup_scan().expect("clean retry");
                assert_eq!(
                    receiver
                        .complete_startup_scan(retry)
                        .expect("retry handoff"),
                    StartupHandoff::Running
                );
            }
            StartupHandoff::Running => assert!(matches!(
                receiver.recv(Duration::from_secs(1)).expect("queued hint"),
                Some(BackendSignal::Hints(batch)) if batch.len() == 1
            )),
        }
        drop(sender);
    }
}

#[test]
fn watcher_capture_first_startup_terminal_is_first_wins_and_never_dirty() {
    let (sender, receiver) = bounded_capture(9);
    let token = receiver.begin_startup_scan().expect("startup pass");
    let first = WatcherFailure::new(
        WatcherFailurePhase::Receive,
        WatcherFailureKind::Panic,
        "first terminal cause",
    );
    sender.terminate(first.clone());
    sender.terminate(WatcherFailure::new(
        WatcherFailurePhase::Receive,
        WatcherFailureKind::Backend,
        "later producer drop",
    ));

    let failure = receiver
        .complete_startup_scan(token)
        .expect_err("terminal must preempt startup handoff");
    assert_eq!(failure, first);
}

#[test]
fn watcher_capture_first_startup_rejects_stale_generation_token() {
    let (_old_sender, old_receiver) = bounded_capture(10);
    let stale = old_receiver.begin_startup_scan().expect("old token");
    let (_new_sender, new_receiver) = bounded_capture(11);

    let failure = new_receiver
        .complete_startup_scan(stale)
        .expect_err("stale generation must fail closed");
    assert_eq!(failure.kind, WatcherFailureKind::Coordination);
    assert!(failure.primary.contains("stale watcher startup generation"));
}

#[test]
fn watcher_capture_first_startup_rejects_stale_scan_pass_token() {
    let (sender, receiver) = bounded_capture(12);
    let stale = receiver.begin_startup_scan().expect("first pass");
    sender.submit(CaptureInput::Reconcile);
    let current = receiver.begin_startup_scan().expect("replacement pass");

    let failure = receiver
        .complete_startup_scan(stale)
        .expect_err("old pass token must not be interpreted as current churn");
    assert_eq!(failure.kind, WatcherFailureKind::Coordination);
    assert!(failure.primary.contains("stale watcher startup scan-pass"));
    assert_eq!(
        receiver
            .complete_startup_scan(current)
            .expect("current pass handoff"),
        StartupHandoff::Running
    );
}

#[test]
fn watcher_bounded_capture() {
    let (sender, receiver) = bounded_capture(1);
    handoff_running(&receiver);
    for index in 0..MAX_QUEUED_HINT_BATCHES {
        sender.submit(CaptureInput::Hints(vec![hint(index)]));
    }
    sender.submit(CaptureInput::Hints(vec![hint(MAX_QUEUED_HINT_BATCHES)]));

    let token = match receiver.recv(Duration::ZERO).expect("receive") {
        Some(BackendSignal::Reconcile(token)) => token,
        other => panic!("queue overflow must request reconcile: {other:?}"),
    };
    assert!(receiver.complete_reconcile(token));

    sender.submit(CaptureInput::Hints(
        (0..MAX_HINTS_PER_BATCH).map(hint).collect(),
    ));
    assert!(matches!(
        receiver.recv(Duration::ZERO).expect("receive"),
        Some(BackendSignal::Hints(hints)) if hints.len() == MAX_HINTS_PER_BATCH
    ));

    sender.submit(CaptureInput::Hints(
        (0..=MAX_HINTS_PER_BATCH).map(hint).collect(),
    ));
    assert!(matches!(
        receiver.recv(Duration::ZERO).expect("receive"),
        Some(BackendSignal::Reconcile(_))
    ));
}

#[test]
fn oversized_path_payload_requests_reconcile_without_partial_batch() {
    let (sender, receiver) = bounded_capture(1);
    handoff_running(&receiver);
    let exact =
        FsEventPath::new("a".repeat(MAX_HINT_PATH_BYTES)).expect("flat relative path is valid");
    sender.submit(CaptureInput::Hints(vec![FsEventHint::changed(exact)]));
    assert!(matches!(
        receiver.recv(Duration::ZERO).expect("receive"),
        Some(BackendSignal::Hints(hints)) if hints.len() == 1
    ));

    let path =
        FsEventPath::new("a".repeat(MAX_HINT_PATH_BYTES + 1)).expect("flat relative path is valid");
    sender.submit(CaptureInput::Hints(vec![FsEventHint::changed(path)]));
    assert!(matches!(
        receiver.recv(Duration::ZERO).expect("receive"),
        Some(BackendSignal::Reconcile(_))
    ));
}

#[test]
fn reconcile_latch_clears_only_after_clean_full_reconcile() {
    let (sender, receiver) = bounded_capture(1);
    handoff_running(&receiver);
    sender.submit(CaptureInput::Reconcile);
    let first = match receiver.recv(Duration::ZERO).expect("receive") {
        Some(BackendSignal::Reconcile(token)) => token,
        other => panic!("expected reconcile token: {other:?}"),
    };

    sender.submit(CaptureInput::Hints(vec![hint(1)]));
    assert!(!receiver.complete_reconcile(first));
    let second = match receiver.recv(Duration::ZERO).expect("receive") {
        Some(BackendSignal::Reconcile(token)) => token,
        other => panic!("dirty reconcile must remain latched: {other:?}"),
    };
    assert!(receiver.complete_reconcile(second));
    assert!(
        receiver
            .recv(Duration::from_millis(1))
            .expect("receive")
            .is_none(),
        "raw hints observed while reconciling must not be replayed"
    );
}

#[test]
fn reconcile_waits_for_inflight_producer_before_draining() {
    let (sender, receiver) = bounded_capture(1);
    handoff_running(&receiver);
    let producer = match sender.gate.prepare_hint() {
        HintDisposition::Queue(claim) => claim,
        HintDisposition::Captured => panic!("running producer guard"),
    };
    sender.tx.try_send(vec![hint(1)]).expect("queue hint");
    sender.gate.mark_dirty();
    let (done_tx, done_rx) = mpsc::channel();
    let join = std::thread::spawn(move || {
        done_tx
            .send(receiver.recv(Duration::from_secs(1)))
            .expect("send result");
    });

    assert!(
        done_rx.recv_timeout(Duration::from_millis(10)).is_err(),
        "consumer must not cross an in-flight producer"
    );
    drop(producer);
    assert!(matches!(
        done_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("consumer result")
            .expect("receive"),
        Some(BackendSignal::Reconcile(_))
    ));
    join.join().expect("consumer thread");
}

#[test]
fn reconcile_waits_for_inflight_dispatch_before_draining() {
    let (sender, receiver) = bounded_capture(1);
    handoff_running(&receiver);
    sender.submit(CaptureInput::Hints(vec![hint(1)]));
    let batch = match receiver.recv(Duration::ZERO).expect("receive") {
        Some(BackendSignal::Hints(batch)) => batch,
        other => panic!("expected dispatch batch: {other:?}"),
    };
    sender.submit(CaptureInput::Reconcile);
    let (done_tx, done_rx) = mpsc::channel();
    let join = std::thread::spawn(move || {
        done_tx
            .send(receiver.recv(Duration::from_secs(1)))
            .expect("send result");
    });

    assert!(
        done_rx.recv_timeout(Duration::from_millis(10)).is_err(),
        "reconcile must not cross an in-flight dispatch"
    );
    drop(batch);
    assert!(matches!(
        done_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("consumer result")
            .expect("receive"),
        Some(BackendSignal::Reconcile(_))
    ));
    join.join().expect("consumer thread");
}

#[test]
fn watcher_final_state_shutdown_discards_all_queued_hints() {
    let (sender, receiver) = bounded_capture(43);
    let token = receiver.begin_startup_scan().expect("startup token");
    assert_eq!(
        receiver
            .complete_startup_scan(token)
            .expect("running handoff"),
        StartupHandoff::Running
    );
    for path in ["notes/first.md", "notes/second.md"] {
        sender.submit(CaptureInput::Hints(vec![FsEventHint::changed(
            FsEventPath::new(path.into()).expect("test path"),
        )]));
    }

    receiver.discard_pending_hints();

    assert!(
        receiver
            .recv(Duration::from_millis(1))
            .expect("capture receive")
            .is_none(),
        "shutdown must discard the entire normalized hint suffix"
    );
}

#[test]
fn terminal_sender_drop_preempts_queued_hints() {
    let (sender, receiver) = bounded_capture(1);
    handoff_running(&receiver);
    sender.submit(CaptureInput::Hints(vec![hint(1)]));
    sender.submit(CaptureInput::Hints(vec![hint(2)]));
    drop(sender);

    assert!(matches!(
        receiver.recv(Duration::ZERO).expect("receive"),
        Some(BackendSignal::Terminal(_))
    ));
    assert!(
        receiver.rx.try_recv().is_err(),
        "queued hints must be discarded"
    );
}
