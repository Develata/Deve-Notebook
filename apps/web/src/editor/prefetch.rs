use deve_core::models::Op;
use gloo_timers::callback::Timeout;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PrefetchConfig {
    pub target_ms: f64,
    pub initial_batch: usize,
    pub max_batch: usize,
}

/// 批量应用操作的回调类型
pub type ApplyBatchFn = Rc<dyn Fn(&[(u64, Op)])>;

pub fn apply_ops_in_batches(
    ops: Vec<(u64, Op)>,
    config: PrefetchConfig,
    apply_batch: ApplyBatchFn,
    on_progress: Rc<dyn Fn(usize, usize, f64)>,
    on_done: Rc<dyn Fn()>,
) {
    if ops.is_empty() {
        on_done();
        return;
    }

    let ops = Rc::new(ops);
    let state = Rc::new(RefCell::new(BatchState {
        index: 0,
        batch: config.initial_batch.max(1),
        max_batch: config.max_batch.max(1),
        target_ms: config.target_ms.max(1.0),
    }));

    schedule_batch(ops, state, apply_batch, on_progress, on_done);
}

struct BatchState {
    index: usize,
    batch: usize,
    max_batch: usize,
    target_ms: f64,
}

fn schedule_batch(
    ops: Rc<Vec<(u64, Op)>>,
    state: Rc<RefCell<BatchState>>,
    apply_batch: ApplyBatchFn,
    on_progress: Rc<dyn Fn(usize, usize, f64)>,
    on_done: Rc<dyn Fn()>,
) {
    let task = move || run_batch(ops, state, apply_batch, on_progress, on_done);
    Timeout::new(0, task).forget();
}

fn run_batch(
    ops: Rc<Vec<(u64, Op)>>,
    state: Rc<RefCell<BatchState>>,
    apply_batch: ApplyBatchFn,
    on_progress: Rc<dyn Fn(usize, usize, f64)>,
    on_done: Rc<dyn Fn()>,
) {
    let total = ops.len();
    let mut st = state.borrow_mut();
    if st.index >= total {
        drop(st);
        on_done();
        return;
    }

    let start = now_ms();
    let remaining = total - st.index;
    let count = st.batch.min(remaining);
    let start_idx = st.index;
    let end_idx = st.index + count;
    apply_batch(&ops[start_idx..end_idx]);
    st.index = end_idx;

    let elapsed = now_ms() - start;
    on_progress(st.index, total, elapsed);
    if elapsed > st.target_ms {
        st.batch = (st.batch / 2).max(1);
    } else if elapsed < st.target_ms * 0.6 {
        st.batch = (st.batch + 4).min(st.max_batch);
    }

    drop(st);
    schedule_batch(ops, state, apply_batch, on_progress, on_done);
}

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}
