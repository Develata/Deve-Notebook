//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 15_release#runtime-observability
//!
use super::metrics::DiffMetricsState;
use super::model::{LineView, UnifiedLine};
use leptos::prelude::*;

mod compute;
pub use compute::create_compute_state;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComputePhase {
    Computing,
    PartialReady,
    Ready,
}

#[derive(Clone)]
pub struct DiffComputeState {
    pub is_editing: ReadSignal<bool>,
    pub set_is_editing: WriteSignal<bool>,
    pub content: ReadSignal<String>,
    pub set_content: WriteSignal<String>,
    pub compute_state: ReadSignal<ComputePhase>,
    pub diff_result: Memo<(Vec<LineView>, Vec<LineView>)>,
    pub unified_lines: Memo<Vec<UnifiedLine>>,
    pub metrics: DiffMetricsState,
}
