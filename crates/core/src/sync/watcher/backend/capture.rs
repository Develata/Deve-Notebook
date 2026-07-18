//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::{
    BackendHintBatch, BackendSignal, FsEventHint, MAX_HINT_PATH_BYTES, MAX_HINTS_PER_BATCH,
    MAX_QUEUED_HINT_BATCHES, ReconcileToken, StartupHandoff, StartupScanToken,
};
use crate::sync::watcher::{WatcherError, WatcherFailure, WatcherFailureKind, WatcherFailurePhase};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const MODE_MASK: u64 = 0b11;
const MODE_CAPTURE_ONLY: u64 = 0;
const MODE_RUNNING: u64 = 1;
const MODE_RECONCILING: u64 = 2;
const MODE_TERMINAL: u64 = 3;
const INFLIGHT_ONE: u64 = 1 << 2;
const STARTUP_DIRTY: u64 = 1 << 2;
const STARTUP_PASS_ONE: u64 = 1 << 3;
const STARTUP_PASS_MASK: u64 = !(MODE_MASK | STARTUP_DIRTY);

// During startup the high bits are the exact pass identity and bit 2 is its
// level-triggered dirty latch. The successful clean-token -> MODE_RUNNING CAS
// is the cut: a competing callback either dirties that pass first or observes
// Running and enters the bounded queue.
// After the cut the same high bits are reused only for Running in-flight claims.

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

struct CaptureGate {
    generation: u64,
    phase_and_inflight: AtomicU64,
    dirty_epoch: AtomicU64,
    clean_epoch: AtomicU64,
    terminal_failure: Mutex<Option<WatcherFailure>>,
}

pub(super) struct RunningClaim {
    gate: Arc<CaptureGate>,
}

enum HintDisposition {
    Queue(RunningClaim),
    Captured,
}

impl Drop for RunningClaim {
    fn drop(&mut self) {
        self.gate
            .phase_and_inflight
            .fetch_sub(INFLIGHT_ONE, Ordering::AcqRel);
    }
}

impl CaptureGate {
    fn begin_startup_scan(&self) -> Result<StartupScanToken, WatcherFailure> {
        loop {
            let state = self.phase_and_inflight.load(Ordering::Acquire);
            match state & MODE_MASK {
                MODE_CAPTURE_ONLY => {
                    let pass = state & STARTUP_PASS_MASK;
                    let Some(next_pass) = pass.checked_add(STARTUP_PASS_ONE) else {
                        return Err(startup_state_failure("startup scan token exhausted"));
                    };
                    let next = next_pass | MODE_CAPTURE_ONLY;
                    if self
                        .phase_and_inflight
                        .compare_exchange_weak(state, next, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                    {
                        return Ok(StartupScanToken::new(next, self.generation));
                    }
                }
                MODE_TERMINAL => return Err(self.terminal_failure()),
                _ => {
                    return Err(startup_state_failure(
                        "startup scan requested after handoff",
                    ));
                }
            }
        }
    }

    fn complete_startup_scan(
        &self,
        token: StartupScanToken,
    ) -> Result<StartupHandoff, WatcherFailure> {
        if token.generation != self.generation {
            return Err(startup_state_failure(format!(
                "stale watcher startup generation: expected {}, found {}",
                self.generation, token.generation
            )));
        }
        loop {
            let state = self.phase_and_inflight.load(Ordering::Acquire);
            if state == token.state {
                if self
                    .phase_and_inflight
                    .compare_exchange(
                        token.state,
                        MODE_RUNNING,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_ok()
                {
                    return Ok(StartupHandoff::Running);
                }
                continue;
            }
            return match state & MODE_MASK {
                MODE_CAPTURE_ONLY
                    if state & STARTUP_PASS_MASK == token.state & STARTUP_PASS_MASK
                        && state & STARTUP_DIRTY != 0 =>
                {
                    Ok(StartupHandoff::Dirty)
                }
                MODE_CAPTURE_ONLY => Err(startup_state_failure(
                    "stale watcher startup scan-pass token",
                )),
                MODE_TERMINAL => Err(self.terminal_failure()),
                _ => Err(startup_state_failure(
                    "startup handoff repeated after Running cut",
                )),
            };
        }
    }

    fn mark_dirty(&self) {
        loop {
            let state = self.phase_and_inflight.load(Ordering::Acquire);
            match state & MODE_MASK {
                MODE_CAPTURE_ONLY => {
                    if state & STARTUP_DIRTY != 0 {
                        return;
                    }
                    let next = state | STARTUP_DIRTY;
                    if self
                        .phase_and_inflight
                        .compare_exchange_weak(state, next, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                    {
                        return;
                    }
                }
                MODE_RUNNING | MODE_RECONCILING => {
                    let _ = self.dirty_epoch.fetch_update(
                        Ordering::AcqRel,
                        Ordering::Acquire,
                        |epoch| Some(epoch.saturating_add(1).max(1)),
                    );
                    return;
                }
                MODE_TERMINAL => return,
                _ => unreachable!("capture mode uses the low two state bits"),
            }
        }
    }

    fn is_dirty(&self) -> bool {
        self.dirty_epoch.load(Ordering::Acquire) != self.clean_epoch.load(Ordering::Acquire)
    }

    fn is_terminal(&self) -> bool {
        self.phase_and_inflight.load(Ordering::Acquire) & MODE_MASK == MODE_TERMINAL
    }

    fn mark_terminal(&self, failure: WatcherFailure) {
        // Publish the first payload before the AcqRel terminal transition. A
        // reader that observes MODE_TERMINAL with Acquire can therefore recover
        // the original typed cause; later Drop/cleanup paths cannot overwrite it.
        let mut terminal_failure = match self.terminal_failure.lock() {
            Ok(failure) => failure,
            Err(poisoned) => poisoned.into_inner(),
        };
        if terminal_failure.is_none() {
            *terminal_failure = Some(failure);
        }
        drop(terminal_failure);
        let _ =
            self.phase_and_inflight
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |state| {
                    Some((state & !MODE_MASK) | MODE_TERMINAL)
                });
    }

    fn terminal_failure(&self) -> WatcherFailure {
        let terminal_failure = match self.terminal_failure.lock() {
            Ok(failure) => failure,
            Err(poisoned) => poisoned.into_inner(),
        };
        terminal_failure.clone().unwrap_or_else(|| {
            startup_state_failure("terminal capture state has no recorded failure")
        })
    }

    fn prepare_hint(self: &Arc<Self>) -> HintDisposition {
        loop {
            let state = self.phase_and_inflight.load(Ordering::Acquire);
            match state & MODE_MASK {
                MODE_CAPTURE_ONLY => {
                    if state & STARTUP_DIRTY != 0 {
                        return HintDisposition::Captured;
                    }
                    let next = state | STARTUP_DIRTY;
                    if self
                        .phase_and_inflight
                        .compare_exchange_weak(state, next, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                    {
                        return HintDisposition::Captured;
                    }
                }
                MODE_RUNNING => {
                    if self.is_dirty() || state > u64::MAX - INFLIGHT_ONE {
                        self.mark_dirty();
                        return HintDisposition::Captured;
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
                            self.mark_dirty();
                            return HintDisposition::Captured;
                        }
                        return HintDisposition::Queue(claim);
                    }
                }
                MODE_RECONCILING => {
                    self.mark_dirty();
                    return HintDisposition::Captured;
                }
                MODE_TERMINAL => return HintDisposition::Captured,
                _ => unreachable!("capture mode uses the low two state bits"),
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

pub(super) fn bounded_capture(generation: u64) -> (CaptureSender, CaptureReceiver) {
    let (tx, rx) = sync_channel(MAX_QUEUED_HINT_BATCHES);
    let gate = Arc::new(CaptureGate {
        generation,
        phase_and_inflight: AtomicU64::new(MODE_CAPTURE_ONLY),
        dirty_epoch: AtomicU64::new(0),
        clean_epoch: AtomicU64::new(0),
        terminal_failure: Mutex::new(None),
    });
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

    pub(super) fn terminate(&self, failure: WatcherFailure) {
        self.gate.mark_terminal(failure);
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
        let _producer = match self.gate.prepare_hint() {
            HintDisposition::Queue(claim) => claim,
            HintDisposition::Captured => return,
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
        self.gate.mark_terminal(WatcherFailure::new(
            WatcherFailurePhase::Receive,
            WatcherFailureKind::Backend,
            "watcher backend producer stopped",
        ));
    }
}

impl CaptureReceiver {
    pub(super) fn begin_startup_scan(&self) -> Result<StartupScanToken, WatcherFailure> {
        self.gate.begin_startup_scan()
    }

    pub(super) fn complete_startup_scan(
        &self,
        token: StartupScanToken,
    ) -> Result<StartupHandoff, WatcherFailure> {
        self.gate.complete_startup_scan(token)
    }

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
                self.gate.mark_terminal(WatcherFailure::new(
                    WatcherFailurePhase::Receive,
                    WatcherFailureKind::Backend,
                    "watcher capture channel disconnected",
                ));
                self.discard_queued_hints();
                Ok(Some(BackendSignal::Terminal(self.gate.terminal_failure())))
            }
        }
    }

    pub(super) fn complete_reconcile(&self, token: ReconcileToken) -> bool {
        self.gate.finish_reconcile(token)
    }

    pub(super) fn discard_pending_hints(&self) {
        self.discard_queued_hints();
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
        match self.gate.prepare_hint() {
            HintDisposition::Queue(claim) => {
                if self.gate.is_terminal() {
                    drop(claim);
                    self.terminal_signal()
                } else {
                    Some(BackendSignal::Hints(BackendHintBatch::new(hints, claim)))
                }
            }
            HintDisposition::Captured => self.begin_reconcile_signal(),
        }
    }

    fn terminal_signal(&self) -> Option<BackendSignal> {
        if !self.gate.is_terminal() {
            return None;
        }
        self.discard_queued_hints();
        Some(BackendSignal::Terminal(self.gate.terminal_failure()))
    }

    fn discard_queued_hints(&self) {
        while self.rx.try_recv().is_ok() {}
    }
}

fn startup_state_failure(detail: impl Into<String>) -> WatcherFailure {
    WatcherFailure::new(
        WatcherFailurePhase::InitialScan,
        WatcherFailureKind::Coordination,
        detail,
    )
}

#[cfg(test)]
#[path = "capture_tests.rs"]
mod tests;
