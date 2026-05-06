//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 15_release#runtime-observability
//!
use super::super::metrics::{create_metrics_state, elapsed_ms, now_ms, record_cache_sample};
use super::super::model::{DiffChunkJob, to_unified};
use super::{ComputePhase, DiffComputeState};
use crate::components::diff_view::cache::DiffLines;
use gloo_timers::callback::Timeout;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[path = "state_compute_helpers.rs"]
mod helpers;
use helpers::{algo_label, cache_completed_diff, initial_cached_or_preview, preview_diff};

struct ChunkedDiffRequest {
    key: String,
    token: u64,
    started_at: u64,
    latest: ReadSignal<u64>,
    set_result: WriteSignal<DiffLines>,
    set_phase: WriteSignal<ComputePhase>,
    metrics: super::super::metrics::DiffMetricsState,
}

fn schedule_chunked_diff(
    delay_ms: u32,
    job: Rc<RefCell<Option<DiffChunkJob>>>,
    timer_ref: Rc<RefCell<Option<Timeout>>>,
    request: Rc<ChunkedDiffRequest>,
) {
    let next_job = job.clone();
    let next_timer_ref = timer_ref.clone();
    let next_request = request.clone();
    let timer = Timeout::new(delay_ms, move || {
        if request.latest.get_untracked() != request.token {
            return;
        }
        let done = {
            let mut job_ref = job.borrow_mut();
            let Some(job) = job_ref.as_mut() else {
                return;
            };
            job.step()
        };

        if done {
            let Some(job) = job.borrow_mut().take() else {
                return;
            };
            let value = job.finish();
            if request.latest.get_untracked() == request.token {
                let algo = value.1;
                cache_completed_diff(request.key.clone(), value.clone());
                record_cache_sample(&request.metrics, false);
                request
                    .metrics
                    .set_algorithm
                    .set(algo_label(algo).to_string());
                request
                    .metrics
                    .set_last_compute_ms
                    .set(elapsed_ms(request.started_at, now_ms()));
                request.set_result.set(value.0);
                request.set_phase.set(ComputePhase::Ready);
            }
            return;
        }

        schedule_chunked_diff(0, next_job, next_timer_ref, next_request);
    });
    *timer_ref.borrow_mut() = Some(timer);
}

fn start_chunked_diff(
    key: String,
    old_content: String,
    new_content: String,
    token: u64,
    delay_ms: u32,
    compute_timer: Rc<RefCell<Option<Timeout>>>,
    latest: ReadSignal<u64>,
    set_result: WriteSignal<DiffLines>,
    set_phase: WriteSignal<ComputePhase>,
    metrics: super::super::metrics::DiffMetricsState,
) {
    let job = Rc::new(RefCell::new(Some(
        super::super::model::create_diff_chunk_job(old_content, new_content),
    )));
    let request = Rc::new(ChunkedDiffRequest {
        key,
        token,
        started_at: now_ms(),
        latest,
        set_result,
        set_phase,
        metrics,
    });
    schedule_chunked_diff(delay_ms, job, compute_timer, request);
}

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
        start_chunked_diff(
            initial_key,
            old_content.as_str().to_string(),
            new_content.clone(),
            initial_token,
            32,
            compute_timer.clone(),
            active_token,
            set_diff_result_raw,
            set_compute_state,
            metrics.clone(),
        );
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

                let key = crate::components::diff_view::cache::build_key(
                    &repo_scope,
                    &path,
                    old_content_ref.as_str(),
                    &text,
                    mode,
                    context_lines,
                );
                if let Some((computed, algo)) = crate::components::diff_view::cache::cache_get(&key)
                {
                    if latest.get_untracked() == next_token {
                        record_cache_sample(&metrics, true);
                        metrics.set_algorithm.set(algo_label(algo).to_string());
                        metrics.set_last_compute_ms.set(0);
                        set_result.set(computed);
                        set_phase.set(ComputePhase::Ready);
                    }
                    return;
                }
                let (preview, preview_algo) = preview_diff(old_content_ref.as_str(), &text);
                if latest.get_untracked() != next_token {
                    return;
                }
                metrics
                    .set_algorithm
                    .set(algo_label(preview_algo).to_string());
                set_result.set(preview);
                set_phase.set(ComputePhase::PartialReady);
                start_chunked_diff(
                    key,
                    old_content_ref.as_str().to_string(),
                    text,
                    next_token,
                    0,
                    compute_timer_ref.clone(),
                    latest,
                    set_result,
                    set_phase,
                    metrics.clone(),
                );
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
