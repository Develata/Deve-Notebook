//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 15_release#runtime-observability
//!
use super::super::super::cache::{DiffLines, build_key, cache_get};
use super::super::super::metrics::{DiffMetricsState, elapsed_ms, now_ms, record_cache_sample};
use super::super::super::model::{DiffChunkJob, create_diff_chunk_job};
use super::super::ComputePhase;
use super::helpers::{algo_label, cache_completed_diff};
use gloo_timers::callback::Timeout;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

struct ChunkedDiffRequest {
    key: String,
    token: u64,
    started_at: u64,
    latest: ReadSignal<u64>,
    set_result: WriteSignal<DiffLines>,
    set_phase: WriteSignal<ComputePhase>,
    metrics: DiffMetricsState,
}

pub(super) struct ChunkedDiffStart {
    pub key: Option<String>,
    pub repo_scope: String,
    pub path: String,
    pub mode: &'static str,
    pub context_lines: usize,
    pub old_content: String,
    pub new_content: String,
    pub token: u64,
    pub delay_ms: u32,
    pub compute_timer: Rc<RefCell<Option<Timeout>>>,
    pub latest: ReadSignal<u64>,
    pub set_result: WriteSignal<DiffLines>,
    pub set_phase: WriteSignal<ComputePhase>,
    pub metrics: DiffMetricsState,
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

pub(super) fn start_chunked_diff(start: ChunkedDiffStart) {
    let started_at = now_ms();
    let delay_ms = start.delay_ms;
    let compute_timer = start.compute_timer.clone();
    let timer_ref = start.compute_timer.clone();
    let timer = Timeout::new(delay_ms, move || {
        if start.latest.get_untracked() != start.token {
            return;
        }
        let key = start.key.unwrap_or_else(|| {
            build_key(
                &start.repo_scope,
                &start.path,
                &start.old_content,
                &start.new_content,
                start.mode,
                start.context_lines,
            )
        });
        if let Some((computed, algo)) = cache_get(&key) {
            if start.latest.get_untracked() == start.token {
                record_cache_sample(&start.metrics, true);
                start
                    .metrics
                    .set_algorithm
                    .set(algo_label(algo).to_string());
                start.metrics.set_last_compute_ms.set(0);
                start.set_result.set(computed);
                start.set_phase.set(ComputePhase::Ready);
            }
            return;
        }

        let job = Rc::new(RefCell::new(Some(create_diff_chunk_job(
            start.old_content,
            start.new_content,
        ))));
        let request = Rc::new(ChunkedDiffRequest {
            key,
            token: start.token,
            started_at,
            latest: start.latest,
            set_result: start.set_result,
            set_phase: start.set_phase,
            metrics: start.metrics,
        });
        schedule_chunked_diff(0, job, timer_ref, request);
    });
    *compute_timer.borrow_mut() = Some(timer);
}
