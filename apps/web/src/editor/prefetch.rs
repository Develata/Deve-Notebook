//! plan_ref:
//!   - 10_rendering#large-document-runtime
//!   - 10_rendering#document-authority-bridge
//!
use gloo_timers::future::TimeoutFuture;
use leptos::task::spawn_local;
use std::rc::Rc;

pub struct PrefetchConfig {
    pub target_ms: f64,
    pub initial_batch: usize,
    pub max_batch: usize,
}

/// 批量应用操作的回调类型
pub type ApplyBatchFn<T> = Rc<dyn Fn(&[T]) -> bool>;
pub type CancelBatchFn<T> = Rc<dyn Fn(&[T])>;

pub fn apply_ops_in_batches<T: 'static>(
    ops: Vec<T>,
    config: PrefetchConfig,
    apply_batch: ApplyBatchFn<T>,
    on_cancel: CancelBatchFn<T>,
    on_progress: Rc<dyn Fn(usize, usize, f64)>,
    on_done: Rc<dyn Fn()>,
) {
    if ops.is_empty() {
        on_done();
        return;
    }

    let mut state = BatchState::new(config);

    spawn_local(async move {
        loop {
            TimeoutFuture::new(0).await;
            let outcome = run_batch(&ops, &mut state, &apply_batch, &on_progress);
            if !settle_batch_outcome(outcome, &ops, &on_cancel, &on_done) {
                break;
            }
        }
    });
}

struct BatchState {
    index: usize,
    batch: usize,
    max_batch: usize,
    target_ms: f64,
}

impl BatchState {
    fn new(config: PrefetchConfig) -> Self {
        let max_batch = config.max_batch.max(1);
        Self {
            index: 0,
            batch: config.initial_batch.max(1).min(max_batch),
            max_batch,
            target_ms: config.target_ms.max(1.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BatchOutcome {
    Continue,
    Complete,
    Cancelled,
}

fn settle_batch_outcome<T>(
    outcome: BatchOutcome,
    ops: &[T],
    on_cancel: &CancelBatchFn<T>,
    on_done: &Rc<dyn Fn()>,
) -> bool {
    match outcome {
        BatchOutcome::Continue => true,
        BatchOutcome::Complete => {
            on_done();
            false
        }
        BatchOutcome::Cancelled => {
            on_cancel(ops);
            false
        }
    }
}

fn run_batch<T>(
    ops: &[T],
    state: &mut BatchState,
    apply_batch: &ApplyBatchFn<T>,
    on_progress: &Rc<dyn Fn(usize, usize, f64)>,
) -> BatchOutcome {
    let total = ops.len();
    if state.index >= total {
        return BatchOutcome::Complete;
    }

    let start = now_ms();
    let remaining = total - state.index;
    let count = state.batch.min(remaining);
    let start_idx = state.index;
    let end_idx = state.index + count;
    if !apply_batch(&ops[start_idx..end_idx]) {
        return BatchOutcome::Cancelled;
    }
    state.index = end_idx;

    let elapsed = now_ms() - start;
    on_progress(state.index, total, elapsed);
    if elapsed > state.target_ms {
        state.batch = (state.batch / 2).max(1);
    } else if elapsed < state.target_ms * 0.6 {
        state.batch = (state.batch + 4).min(state.max_batch);
    }

    if state.index == total {
        BatchOutcome::Complete
    } else {
        BatchOutcome::Continue
    }
}

#[cfg(target_arch = "wasm32")]
fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

#[cfg(not(target_arch = "wasm32"))]
fn now_ms() -> f64 {
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn state(batch: usize) -> BatchState {
        BatchState {
            index: 0,
            batch,
            max_batch: batch,
            target_ms: 8.0,
        }
    }

    #[test]
    fn prefetch_batches_complete_without_an_extra_scheduled_callback() {
        let applied = Rc::new(Cell::new(0usize));
        let applied_for_callback = applied.clone();
        let apply: ApplyBatchFn<u8> = Rc::new(move |batch| {
            applied_for_callback.set(applied_for_callback.get() + batch.len());
            true
        });
        let progress: Rc<dyn Fn(usize, usize, f64)> = Rc::new(|_, _, _| {});
        let mut state = state(2);
        let ops = [1, 2, 3];

        assert_eq!(
            run_batch(&ops, &mut state, &apply, &progress),
            BatchOutcome::Continue
        );
        assert_eq!(
            run_batch(&ops, &mut state, &apply, &progress),
            BatchOutcome::Complete
        );
        assert_eq!(applied.get(), ops.len());
        assert_eq!(state.index, ops.len());
    }

    #[test]
    fn prefetch_batch_failure_cancels_without_advancing_progress() {
        let apply: ApplyBatchFn<u8> = Rc::new(|_| false);
        let progress_calls = Rc::new(Cell::new(0usize));
        let progress_calls_for_callback = progress_calls.clone();
        let progress: Rc<dyn Fn(usize, usize, f64)> = Rc::new(move |_, _, _| {
            progress_calls_for_callback.set(progress_calls_for_callback.get() + 1);
        });
        let mut state = state(2);

        assert_eq!(
            run_batch(&[1, 2], &mut state, &apply, &progress),
            BatchOutcome::Cancelled
        );
        assert_eq!(state.index, 0);
        assert_eq!(progress_calls.get(), 0);
    }

    #[test]
    fn cancelled_batch_routes_retained_ops_only_to_failure_handler() {
        let cancelled_ops = Rc::new(Cell::new(0usize));
        let cancelled_ops_for_callback = cancelled_ops.clone();
        let on_cancel: CancelBatchFn<u8> = Rc::new(move |ops| {
            cancelled_ops_for_callback.set(ops.len());
        });
        let done_calls = Rc::new(Cell::new(0usize));
        let done_calls_for_callback = done_calls.clone();
        let on_done: Rc<dyn Fn()> = Rc::new(move || {
            done_calls_for_callback.set(done_calls_for_callback.get() + 1);
        });

        assert!(!settle_batch_outcome(
            BatchOutcome::Cancelled,
            &[1, 2, 3],
            &on_cancel,
            &on_done,
        ));
        assert_eq!(cancelled_ops.get(), 3);
        assert_eq!(done_calls.get(), 0);
    }

    #[test]
    fn prefetch_initial_batch_respects_configured_maximum() {
        let state = BatchState::new(PrefetchConfig {
            target_ms: 8.0,
            initial_batch: 32,
            max_batch: 4,
        });

        assert_eq!(state.batch, 4);
        assert_eq!(state.max_batch, 4);
    }
}
