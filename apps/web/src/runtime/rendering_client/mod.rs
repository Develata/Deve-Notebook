//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!
//! Browser rendering client runtime.
//!
//! This adapter contains UI/editor/rendering object-plane state. It must not
//! mutate pending writes or decide write success.

use crate::editor::EditorStats;
use leptos::prelude::*;

#[allow(dead_code)]
#[derive(Clone)]
pub struct RenderingClient {
    pub stats: ReadSignal<EditorStats>,
    pub on_stats: Callback<EditorStats>,
    pub load_state: ReadSignal<String>,
    pub set_load_state: WriteSignal<String>,
    pub load_progress: ReadSignal<(usize, usize)>,
    pub set_load_progress: WriteSignal<(usize, usize)>,
    pub load_eta_ms: ReadSignal<u64>,
    pub set_load_eta_ms: WriteSignal<u64>,
}
