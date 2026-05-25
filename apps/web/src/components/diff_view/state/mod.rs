//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 18_release#runtime-observability
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

pub fn diff_compute_indicator_visible(phase: ComputePhase) -> bool {
    phase != ComputePhase::Ready
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

#[cfg(test)]
mod tests {
    use super::{ComputePhase, diff_compute_indicator_visible};

    #[test]
    fn diff_compute_indicator_tracks_non_ready_phases() {
        assert!(diff_compute_indicator_visible(ComputePhase::Computing));
        assert!(diff_compute_indicator_visible(ComputePhase::PartialReady));
        assert!(!diff_compute_indicator_visible(ComputePhase::Ready));
    }
}
