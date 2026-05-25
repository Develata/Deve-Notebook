//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use leptos::prelude::*;

use super::super::super::contexts::SystemMetricsData;

#[derive(Clone, Copy)]
pub(super) struct SyncRuntimeSignals {
    pub load_state: ReadSignal<String>,
    pub set_load_state: WriteSignal<String>,
    pub load_progress: ReadSignal<(usize, usize)>,
    pub set_load_progress: WriteSignal<(usize, usize)>,
    pub load_eta_ms: ReadSignal<u64>,
    pub set_load_eta_ms: WriteSignal<u64>,
    pub sync_mode: ReadSignal<String>,
    pub set_sync_mode: WriteSignal<String>,
    pub sync_mode_request_id: ReadSignal<Option<String>>,
    pub set_sync_mode_request_id: WriteSignal<Option<String>>,
    pub pending_ops_count: ReadSignal<u32>,
    pub set_pending_ops_count: WriteSignal<u32>,
    pub pending_ops_previews: ReadSignal<Vec<(String, String, String)>>,
    pub set_pending_ops_previews: WriteSignal<Vec<(String, String, String)>>,
    pub pending_ops_request_id: ReadSignal<Option<String>>,
    pub set_pending_ops_request_id: WriteSignal<Option<String>>,
    pub system_metrics: ReadSignal<Option<SystemMetricsData>>,
    pub set_system_metrics: WriteSignal<Option<SystemMetricsData>>,
    pub system_metrics_live: ReadSignal<bool>,
    pub set_system_metrics_live: WriteSignal<bool>,
    pub set_explicit_home: WriteSignal<bool>,
}

pub(super) fn init_sync_runtime_signals() -> SyncRuntimeSignals {
    let (load_state, set_load_state) = signal("ready".to_string());
    let (load_progress, set_load_progress) = signal((0usize, 0usize));
    let (load_eta_ms, set_load_eta_ms) = signal(0u64);
    let (sync_mode, set_sync_mode) = signal("auto".to_string());
    let (sync_mode_request_id, set_sync_mode_request_id) = signal(None::<String>);
    let (pending_ops_count, set_pending_ops_count) = signal(0u32);
    let (pending_ops_previews, set_pending_ops_previews) = signal(Vec::new());
    let (pending_ops_request_id, set_pending_ops_request_id) = signal(None::<String>);
    let (system_metrics, set_system_metrics) = signal(None::<SystemMetricsData>);
    let (system_metrics_live, set_system_metrics_live) = signal(false);
    let (_, set_explicit_home) = signal(false);

    SyncRuntimeSignals {
        load_state,
        set_load_state,
        load_progress,
        set_load_progress,
        load_eta_ms,
        set_load_eta_ms,
        sync_mode,
        set_sync_mode,
        sync_mode_request_id,
        set_sync_mode_request_id,
        pending_ops_count,
        set_pending_ops_count,
        pending_ops_previews,
        set_pending_ops_previews,
        pending_ops_request_id,
        set_pending_ops_request_id,
        system_metrics,
        set_system_metrics,
        system_metrics_live,
        set_system_metrics_live,
        set_explicit_home,
    }
}
