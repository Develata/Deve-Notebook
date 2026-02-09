use super::metrics::DiffMetricsState;
use super::model::{LineView, UnifiedLine};
use leptos::prelude::*;

#[path = "state_compute.rs"]
mod state_compute;
pub use state_compute::create_compute_state;

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
