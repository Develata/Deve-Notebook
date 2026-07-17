//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::{
    BackendHintBatch, BackendSignal, FsEventHint, MAX_HINT_PATH_BYTES, MAX_HINTS_PER_BATCH,
    MAX_QUEUED_HINT_BATCHES, ReconcileToken,
};
use crate::sync::watcher::WatcherError;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError, sync_channel};
use std::time::Duration;

const MODE_MASK: u64 = 0b11;
const MODE_RUNNING: u64 = 0;
const MODE_RECONCILING: u64 = 1;
const MODE_TERMINAL: u64 = 3;
const INFLIGHT_ONE: u64 = 1 << 2;

pub(super) struct CaptureSender {
    tx: SyncSender<Vec<FsEventHint>>,
    gate: Arc<CaptureGate>,
}

pub(super) struct CaptureReceiver {
    rx: Receiver<Vec<FsEventHint>>,
    gate: Arc<CaptureGate>,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum CaptureInput {
    Hints(Vec<FsEventHint>),
    Reconcile,
    Ignore,
}

#[derive(Default)]
struct CaptureGate {
    phase_and_inflight: AtomicU64,
    dirty_epoch: AtomicU64,
    clean_epoch: AtomicU64,
}

pub(super) struct RunningClaim {
    gate: Arc<CaptureGate>,
}

impl Drop for RunningClaim {
    fn drop(&mut self) {
        self.gate
            .phase_and_inflight
            .fetch_sub(INFLIGHT_ONE, Ordering::AcqRel);
    }
}

impl CaptureGate {
    fn mark_dirty(&self) {
        let _ = self
            .dirty_epoch
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |epoch| {
                Some(epoch.saturating_add(1).max(1))
            });
    }

    fn is_dirty(&self) -> bool {
        self.dirty_epoch.load(Ordering::Acquire) != self.clean_epoch.load(Ordering::Acquire)
    }

    fn is_terminal(&self) -> bool {
        self.phase_and_inflight.load(Ordering::Acquire) & MODE_MASK == MODE_TERMINAL
    }

    fn mark_terminal(&self) {
        let _ =
            self.phase_and_inflight
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |state| {
                    Some((state & !MODE_MASK) | MODE_TERMINAL)
                });
    }

    fn acquire_running_claim(self: &Arc<Self>) -> Option<RunningClaim> {
        loop {
            if self.is_dirty() {
                return None;
            }
            let state = self.phase_and_inflight.load(Ordering::Acquire);
            if state & MODE_MASK != MODE_RUNNING || state > u64::MAX - INFLIGHT_ONE {
                return None;
            }
            if self
                .phase_and_inflight
                .compare_exchange_weak(
                    state,
                    state + INFLIGHT_ONE,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                let claim = RunningClaim { gate: self.clone() };
                if self.is_dirty() {
                    drop(claim);
                    return None;
                }
                return Some(claim);
            }
        }
    }

    fn begin_reconcile(&self) -> Option<ReconcileToken> {
        if !self.is_dirty() {
            return None;
        }
        loop {
            let state = self.phase_and_inflight.load(Ordering::Acquire);
            if state & MODE_MASK != MODE_RUNNING {
                return None;
            }
            if state != MODE_RUNNING {
                std::thread::yield_now();
                continue;
            }
            if self
                .phase_and_inflight
                .compare_exchange_weak(
                    MODE_RUNNING,
                    MODE_RECONCILING,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                return Some(ReconcileToken(self.dirty_epoch.load(Ordering::Acquire)));
            }
        }
    }

    fn finish_reconcile(&self, token: ReconcileToken) -> bool {
        let dirty_at_finish = self.dirty_epoch.load(Ordering::Acquire);
        let unchanged = dirty_at_finish == token.0 && token.0 != u64::MAX;
        let transitioned = self
            .phase_and_inflight
            .compare_exchange(
                MODE_RECONCILING,
                MODE_RUNNING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok();
        if transitioned && unchanged {
            self.clean_epoch.store(token.0, Ordering::Release);
        }
        transitioned && unchanged && self.dirty_epoch.load(Ordering::Acquire) == token.0
    }
}

pub(super) fn bounded_capture() -> (CaptureSender, CaptureReceiver) {
    let (tx, rx) = sync_channel(MAX_QUEUED_HINT_BATCHES);
    let gate = Arc::new(CaptureGate::default());
    (
        CaptureSender {
            tx,
            gate: gate.clone(),
        },
        CaptureReceiver { rx, gate },
    )
}

impl CaptureSender {
    pub(super) fn submit(&self, input: CaptureInput) {
        match input {
            CaptureInput::Ignore => {}
            CaptureInput::Reconcile => self.gate.mark_dirty(),
            CaptureInput::Hints(hints) => self.submit_hints(hints),
        }
    }

    fn submit_hints(&self, hints: Vec<FsEventHint>) {
        if hints.is_empty() {
            return;
        }
        let path_bytes = hints.iter().try_fold(0usize, |total, hint| {
            total.checked_add(hint.path_payload_bytes())
        });
        if hints.len() > MAX_HINTS_PER_BATCH
            || path_bytes.is_none_or(|bytes| bytes > MAX_HINT_PATH_BYTES)
        {
            self.gate.mark_dirty();
            return;
        }
        let Some(_producer) = self.gate.acquire_running_claim() else {
            self.gate.mark_dirty();
            return;
        };
        match self.tx.try_send(hints) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                self.gate.mark_dirty();
            }
        }
    }
}

impl Drop for CaptureSender {
    fn drop(&mut self) {
        self.gate.mark_terminal();
    }
}

impl CaptureReceiver {
    pub(super) fn recv(&self, timeout: Duration) -> Result<Option<BackendSignal>, WatcherError> {
        if let Some(signal) = self.terminal_signal() {
            return Ok(Some(signal));
        }
        if let Some(signal) = self.begin_reconcile_signal() {
            return Ok(Some(signal));
        }
        match self.rx.recv_timeout(timeout) {
            Ok(hints) => {
                if let Some(signal) = self.terminal_signal() {
                    Ok(Some(signal))
                } else {
                    Ok(self.claim_hints_or_reconcile(hints))
                }
            }
            Err(RecvTimeoutError::Timeout) => Ok(self
                .terminal_signal()
                .or_else(|| self.begin_reconcile_signal())),
            Err(RecvTimeoutError::Disconnected) => {
                self.gate.mark_terminal();
                self.discard_queued_hints();
                Ok(Some(BackendSignal::Terminal))
            }
        }
    }

    pub(super) fn complete_reconcile(&self, token: ReconcileToken) -> bool {
        self.gate.finish_reconcile(token)
    }

    fn begin_reconcile_signal(&self) -> Option<BackendSignal> {
        if let Some(signal) = self.terminal_signal() {
            return Some(signal);
        }
        let token = self.gate.begin_reconcile()?;
        if let Some(signal) = self.terminal_signal() {
            return Some(signal);
        }
        self.discard_queued_hints();
        Some(BackendSignal::Reconcile(token))
    }

    fn claim_hints_or_reconcile(&self, hints: Vec<FsEventHint>) -> Option<BackendSignal> {
        match self.gate.acquire_running_claim() {
            Some(claim) => {
                if self.gate.is_terminal() {
                    drop(claim);
                    self.terminal_signal()
                } else {
                    Some(BackendSignal::Hints(BackendHintBatch::new(hints, claim)))
                }
            }
            None => self.begin_reconcile_signal(),
        }
    }

    fn terminal_signal(&self) -> Option<BackendSignal> {
        if !self.gate.is_terminal() {
            return None;
        }
        self.discard_queued_hints();
        Some(BackendSignal::Terminal)
    }

    fn discard_queued_hints(&self) {
        while self.rx.try_recv().is_ok() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::watcher::backend::FsEventPath;
    use std::sync::mpsc;

    fn hint(index: usize) -> FsEventHint {
        FsEventHint::changed(
            FsEventPath::new(format!("note-{index}.md")).expect("valid relative path"),
        )
    }

    #[test]
    fn watcher_bounded_capture() {
        let (sender, receiver) = bounded_capture();
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
        let (sender, receiver) = bounded_capture();
        let exact =
            FsEventPath::new("a".repeat(MAX_HINT_PATH_BYTES)).expect("flat relative path is valid");
        sender.submit(CaptureInput::Hints(vec![FsEventHint::changed(exact)]));
        assert!(matches!(
            receiver.recv(Duration::ZERO).expect("receive"),
            Some(BackendSignal::Hints(hints)) if hints.len() == 1
        ));

        let path = FsEventPath::new("a".repeat(MAX_HINT_PATH_BYTES + 1))
            .expect("flat relative path is valid");
        sender.submit(CaptureInput::Hints(vec![FsEventHint::changed(path)]));
        assert!(matches!(
            receiver.recv(Duration::ZERO).expect("receive"),
            Some(BackendSignal::Reconcile(_))
        ));
    }

    #[test]
    fn reconcile_latch_clears_only_after_clean_full_reconcile() {
        let (sender, receiver) = bounded_capture();
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
        let (sender, receiver) = bounded_capture();
        let producer = sender.gate.acquire_running_claim().expect("producer guard");
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
        let (sender, receiver) = bounded_capture();
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
    fn terminal_sender_drop_preempts_queued_hints() {
        let (sender, receiver) = bounded_capture();
        sender.submit(CaptureInput::Hints(vec![hint(1)]));
        sender.submit(CaptureInput::Hints(vec![hint(2)]));
        drop(sender);

        assert!(matches!(
            receiver.recv(Duration::ZERO).expect("receive"),
            Some(BackendSignal::Terminal)
        ));
        assert!(
            receiver.rx.try_recv().is_err(),
            "queued hints must be discarded"
        );
    }
}
