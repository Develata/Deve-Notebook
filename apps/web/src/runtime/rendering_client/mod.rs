//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!
//! Browser rendering client runtime.
//!
//! This adapter contains UI/editor/rendering object-plane state. It must not
//! mutate pending writes or decide write success.

use crate::editor::EditorStats;
use crate::runtime::domain::LoadPhase;
use leptos::prelude::*;

#[allow(dead_code)]
#[derive(Clone)]
pub struct RenderingClient {
    pub stats: ReadSignal<EditorStats>,
    pub on_stats: Callback<EditorStats>,
    pub load_state: ReadSignal<LoadPhase>,
    pub set_load_state: WriteSignal<LoadPhase>,
    pub load_progress: ReadSignal<(usize, usize)>,
    pub set_load_progress: WriteSignal<(usize, usize)>,
    pub load_eta_ms: ReadSignal<u64>,
    pub set_load_eta_ms: WriteSignal<u64>,
}
