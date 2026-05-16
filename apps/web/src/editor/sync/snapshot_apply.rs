//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 05_network#web-ws-runtime
//!
use super::snapshot_gate::SnapshotRequestGate;
use crate::editor::ffi::applyRemoteOpsBatch;
use deve_core::models::Op;
use deve_core::protocol::ConfirmedOp;
use leptos::prelude::*;

pub(super) type BatchHandler = std::rc::Rc<dyn Fn(&[ConfirmedOp]) -> bool>;
pub(super) type BatchFailureHandler = std::rc::Rc<dyn Fn()>;
type RemoteBatchApplier = std::rc::Rc<dyn Fn(&[Op]) -> bool>;
pub(super) type ProgressHandler = std::rc::Rc<dyn Fn(usize, usize, f64)>;

#[derive(Clone, Copy)]
pub(super) struct SnapshotApplySignals {
    pub set_local_version: WriteSignal<u64>,
    pub set_history: WriteSignal<Vec<(u64, Op)>>,
}

pub(super) fn build_apply_batch(
    signals: SnapshotApplySignals,
    gate: SnapshotRequestGate,
    on_failure: BatchFailureHandler,
) -> BatchHandler {
    build_apply_batch_with_applier(
        signals,
        gate,
        on_failure,
        std::rc::Rc::new(apply_remote_ops_batch),
    )
}

fn build_apply_batch_with_applier(
    signals: SnapshotApplySignals,
    gate: SnapshotRequestGate,
    on_failure: BatchFailureHandler,
    applier: RemoteBatchApplier,
) -> BatchHandler {
    std::rc::Rc::new(move |batch: &[ConfirmedOp]| {
        if !gate.matches() {
            return false;
        }
        let ops_only: Vec<_> = batch.iter().map(|entry| entry.op.clone()).collect();
        if !applier(&ops_only) {
            on_failure();
            return false;
        }
        if let Some(entry) = batch.last() {
            signals.set_local_version.set(entry.seq);
        }
        signals.set_history.update(|history| {
            for entry in batch {
                history.push((entry.seq, entry.op.clone()));
            }
        });
        true
    })
}

fn apply_remote_ops_batch(ops: &[Op]) -> bool {
    match serde_json::to_string(ops) {
        Ok(json) => applyRemoteOpsBatch(&json),
        Err(err) => {
            leptos::logging::warn!("Snapshot delta batch serialization failed: {err}");
            false
        }
    }
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

#[cfg(test)]
mod tests {
    use super::{SnapshotApplySignals, build_apply_batch_with_applier, build_progress_handler};
    use crate::editor::sync::snapshot_gate::{SnapshotRequestGate, SnapshotRequestGateInput};
    use crate::hooks::use_core::PendingBranchTarget;
    use deve_core::models::Op;
    use deve_core::protocol::ConfirmedOp;
    use leptos::prelude::*;
    use leptos::reactive::owner::Owner;
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::sync::atomic::AtomicU64;

    #[test]
    fn snapshot_apply_failure_does_not_advance_version_or_history() {
        let runtime = Owner::new();
        runtime.set();
        let (local_version, set_local_version) = signal(2u64);
        let (history, set_history) = signal(vec![(
            2,
            Op::Insert {
                pos: 0,
                content: "base".into(),
            },
        )]);
        let failed = Rc::new(Cell::new(false));
        let failed_for_callback = Rc::clone(&failed);
        let handler = build_apply_batch_with_applier(
            SnapshotApplySignals {
                set_local_version,
                set_history,
            },
            matching_gate(),
            Rc::new(move || failed_for_callback.set(true)),
            Rc::new(|_| false),
        );

        assert!(!handler(&[ConfirmedOp::new(
            3,
            Op::Insert {
                pos: 4,
                content: "!".into(),
            },
            None,
        )]));
        assert!(failed.get());
        assert_eq!(local_version.get_untracked(), 2);
        assert_eq!(history.get_untracked().len(), 1);
    }

    #[test]
    fn snapshot_apply_success_advances_version_and_history() {
        let runtime = Owner::new();
        runtime.set();
        let (local_version, set_local_version) = signal(2u64);
        let (history, set_history) = signal(Vec::<(u64, Op)>::new());
        let failed = Rc::new(Cell::new(false));
        let failed_for_callback = Rc::clone(&failed);
        let handler = build_apply_batch_with_applier(
            SnapshotApplySignals {
                set_local_version,
                set_history,
            },
            matching_gate(),
            Rc::new(move || failed_for_callback.set(true)),
            Rc::new(|_| true),
        );

        assert!(handler(&[ConfirmedOp::new(
            5,
            Op::Insert {
                pos: 0,
                content: "ok".into(),
            },
            None,
        )]));
        assert!(!failed.get());
        assert_eq!(local_version.get_untracked(), 5);
        assert_eq!(history.get_untracked().len(), 1);
    }

    #[test]
    fn snapshot_progress_handler_updates_eta() {
        let runtime = Owner::new();
        runtime.set();
        let (progress, set_progress) = signal((0usize, 0usize));
        let (eta, set_eta) = signal(0u64);
        let handler = build_progress_handler(set_progress, set_eta);

        handler(2, 6, 10.0);
        assert_eq!(progress.get_untracked(), (2, 6));
        assert_eq!(eta.get_untracked(), 20);
    }

    fn matching_gate() -> SnapshotRequestGate {
        let repo_id = uuid::Uuid::new_v4();
        let (open_request_id, _) = signal(7u64);
        let (current_repo_id, _) = signal(Some(repo_id.to_string()));
        let (pending_repo_switch, _) = signal(None::<String>);
        let (active_branch, _) = signal(None);
        let (pending_branch_switch, _) = signal(None::<PendingBranchTarget>);
        let (current_scope_nonce, _) = signal(11u64);
        SnapshotRequestGate::new(SnapshotRequestGateInput {
            open_request_id,
            current_repo_id,
            pending_repo_switch,
            active_branch,
            pending_branch_switch,
            current_scope_nonce,
            session_generation: Arc::new(AtomicU64::new(13)),
            expected_generation: 13,
            repo_id,
            branch: None,
            request_id: 7,
            scope_nonce: 11,
        })
    }
}
