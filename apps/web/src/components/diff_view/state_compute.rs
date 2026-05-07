//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 15_release#runtime-observability
//!
use super::super::metrics::{create_metrics_state, record_cache_sample};
use super::super::model::to_unified;
use super::{ComputePhase, DiffComputeState};
use gloo_timers::callback::Timeout;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[path = "state_compute_chunk.rs"]
mod chunk;
#[path = "state_compute_helpers.rs"]
mod helpers;
use chunk::{ChunkedDiffStart, start_chunked_diff};
use helpers::{algo_label, initial_cached_or_preview, preview_diff};

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

    let initial = initial_cached_or_preview(
        &repo_scope,
        &path,
        old_content.as_str(),
        &new_content,
        mode,
        context_lines,
    );
    if initial.complete {
        record_cache_sample(&metrics, initial.cache_hit);
    }
    metrics
        .set_algorithm
        .set(algo_label(initial.value.1).to_string());

    let initial_phase = if initial.complete {
        ComputePhase::Ready
    } else {
        ComputePhase::PartialReady
    };
    let initial_key = initial.key.clone();
    let should_complete_initial = !initial.complete;
    let (diff_result_raw, set_diff_result_raw) = signal(initial.value.0);
    let diff_result = Memo::new(move |_| diff_result_raw.get());
    let (compute_state, set_compute_state) = signal(initial_phase);
    let initial_token = u64::from(should_complete_initial);
    let (active_token, set_active_token) = signal(initial_token);
    let debounce_timer: Rc<RefCell<Option<Timeout>>> = Rc::new(RefCell::new(None));
    let compute_timer: Rc<RefCell<Option<Timeout>>> = Rc::new(RefCell::new(None));
    let metrics_for_effect = metrics.clone();
    let seen_content_effect = Rc::new(RefCell::new(false));

    if should_complete_initial {
        start_chunked_diff(ChunkedDiffStart {
            key: initial_key,
            repo_scope: repo_scope.clone(),
            path: path.clone(),
            mode,
            context_lines,
            old_content: old_content.as_str().to_string(),
            new_content: new_content.clone(),
            token: initial_token,
            delay_ms: 32,
            compute_timer: compute_timer.clone(),
            latest: active_token,
            set_result: set_diff_result_raw,
            set_phase: set_compute_state,
            metrics: metrics.clone(),
        });
    }

    Effect::new({
        let debounce_timer = debounce_timer.clone();
        let compute_timer = compute_timer.clone();
        let old_content = old_content.clone();
        let seen_content_effect = seen_content_effect.clone();
        move |_| {
            let text = content.get();
            {
                let mut seen = seen_content_effect.borrow_mut();
                if !*seen {
                    *seen = true;
                    return;
                }
            }
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

                let (preview, preview_algo) = preview_diff(old_content_ref.as_str(), &text);
                if latest.get_untracked() != next_token {
                    return;
                }
                metrics
                    .set_algorithm
                    .set(algo_label(preview_algo).to_string());
                set_result.set(preview);
                set_phase.set(ComputePhase::PartialReady);
                start_chunked_diff(ChunkedDiffStart {
                    key: None,
                    repo_scope,
                    path,
                    mode,
                    context_lines,
                    old_content: old_content_ref.as_str().to_string(),
                    new_content: text,
                    token: next_token,
                    delay_ms: 0,
                    compute_timer: compute_timer_ref.clone(),
                    latest,
                    set_result,
                    set_phase,
                    metrics: metrics.clone(),
                });
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
