use super::snapshot_gate::SnapshotRequestGate;
use crate::editor::ffi::applyRemoteOpsBatch;
use deve_core::models::Op;
use deve_core::protocol::ConfirmedOp;
use leptos::prelude::*;

pub(super) type BatchHandler = std::rc::Rc<dyn Fn(&[ConfirmedOp])>;
pub(super) type ProgressHandler = std::rc::Rc<dyn Fn(usize, usize, f64)>;

#[derive(Clone, Copy)]
pub(super) struct SnapshotApplySignals {
    pub set_local_version: WriteSignal<u64>,
    pub set_history: WriteSignal<Vec<(u64, Op)>>,
}

pub(super) fn build_apply_batch(
    signals: SnapshotApplySignals,
    gate: SnapshotRequestGate,
) -> BatchHandler {
    std::rc::Rc::new(move |batch: &[ConfirmedOp]| {
        if !gate.matches() {
            return;
        }
        let ops_only: Vec<_> = batch.iter().map(|entry| entry.op.clone()).collect();
        if let Ok(json) = serde_json::to_string(&ops_only) {
            applyRemoteOpsBatch(&json);
        }
        if let Some(entry) = batch.last() {
            signals.set_local_version.set(entry.seq);
        }
        signals.set_history.update(|history| {
            for entry in batch {
                history.push((entry.seq, entry.op.clone()));
            }
        });
    })
}

pub(super) fn build_progress_handler(
    set_load_progress: WriteSignal<(usize, usize)>,
    set_load_eta_ms: WriteSignal<u64>,
) -> ProgressHandler {
    let elapsed_total = std::rc::Rc::new(std::cell::RefCell::new(0.0));
    std::rc::Rc::new(move |done: usize, total: usize, batch_ms: f64| {
        set_load_progress.set((done, total));
        *elapsed_total.borrow_mut() += batch_ms;
        if done > 0 {
            let per_op = *elapsed_total.borrow() / done as f64;
            let remaining = (total - done) as f64 * per_op;
            set_load_eta_ms.set(remaining as u64);
        }
    })
}
