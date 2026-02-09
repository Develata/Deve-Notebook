use super::super::metrics::{create_metrics_state, elapsed_ms, now_ms, record_cache_sample};
use super::super::model::to_unified;
use super::{ComputePhase, DiffComputeState};
use gloo_timers::callback::Timeout;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[path = "state_compute_helpers.rs"]
mod helpers;
use helpers::{algo_label, initial_with_cache, recompute_with_cache};

pub fn create_compute_state(
    repo_scope: String,
    path: String,
    mode: &'static str,
    context_lines: usize,
    old_content: String,
    new_content: String,
) -> DiffComputeState {
    let (is_editing, set_is_editing) = signal(false);
    let (content, set_content) = signal(new_content.clone());
    let metrics = create_metrics_state();
    let old_content = Rc::new(old_content);

    let (hit, (initial, initial_algo)) = initial_with_cache(
        &repo_scope,
        &path,
        old_content.as_str(),
        &new_content,
        mode,
        context_lines,
    );
    record_cache_sample(&metrics, hit);
    metrics
        .set_algorithm
        .set(algo_label(initial_algo).to_string());

    let (diff_result_raw, set_diff_result_raw) = signal(initial);
    let diff_result = Memo::new(move |_| diff_result_raw.get());
    let (compute_state, set_compute_state) = signal(ComputePhase::Ready);
    let (active_token, set_active_token) = signal(0u64);
    let debounce_timer: Rc<RefCell<Option<Timeout>>> = Rc::new(RefCell::new(None));
    let compute_timer: Rc<RefCell<Option<Timeout>>> = Rc::new(RefCell::new(None));
    let metrics_for_effect = metrics.clone();

    Effect::new({
        let debounce_timer = debounce_timer.clone();
        let compute_timer = compute_timer.clone();
        let old_content = old_content.clone();
        move |_| {
            let text = content.get();
            set_compute_state.set(ComputePhase::Computing);
            let next_token = active_token.get_untracked().wrapping_add(1);
            set_active_token.set(next_token);

            if let Some(t) = debounce_timer.borrow_mut().take() {
                t.cancel();
            }
            if let Some(t) = compute_timer.borrow_mut().take() {
                t.cancel();
            }
            let latest = active_token;
            let set_phase = set_compute_state;
            let set_result = set_diff_result_raw;
            let metrics = metrics_for_effect.clone();
            let compute_timer_ref = compute_timer.clone();
            let old_content_ref = old_content.clone();
            let path = path.clone();
            let repo_scope = repo_scope.clone();
            let debounce = Timeout::new(150, move || {
                if latest.get_untracked() != next_token {
                    return;
                }
                set_phase.set(ComputePhase::PartialReady);

                let latest_inner = latest;
                let set_phase_inner = set_phase;
                let set_result_inner = set_result;
                let old_inner = old_content_ref.clone();
                let text_inner = text.clone();
                let metrics_inner = metrics.clone();
                let cache_path = path.clone();
                let compute_job = Timeout::new(0, move || {
                    if latest_inner.get_untracked() != next_token {
                        return;
                    }
                    let start = now_ms();
                    let (hit, (computed, algo)) = recompute_with_cache(
                        &repo_scope,
                        &cache_path,
                        old_inner.as_str(),
                        &text_inner,
                        mode,
                        context_lines,
                    );
                    if latest_inner.get_untracked() == next_token {
                        record_cache_sample(&metrics_inner, hit);
                        metrics_inner
                            .set_algorithm
                            .set(algo_label(algo).to_string());
                        metrics_inner
                            .set_last_compute_ms
                            .set(elapsed_ms(start, now_ms()));
                        set_result_inner.set(computed);
                        set_phase_inner.set(ComputePhase::Ready);
                    }
                });
                *compute_timer_ref.borrow_mut() = Some(compute_job);
            });
            *debounce_timer.borrow_mut() = Some(debounce);
        }
    });

    let unified_lines = Memo::new(move |_| {
        let (left, right) = diff_result.get();
        to_unified(&left, &right)
    });

    DiffComputeState {
        is_editing,
        set_is_editing,
        content,
        set_content,
        compute_state,
        diff_result,
        unified_lines,
        metrics,
    }
}
