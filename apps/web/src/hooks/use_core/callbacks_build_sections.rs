//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::api::WsService;

use super::super::callbacks::{
    DocCallbackSignals, DocCallbacks, MiscCallbacks, create_doc_callbacks, create_misc_callbacks,
};
use super::super::callbacks_sc::{SourceControlCallbacks, create_source_control_callbacks};
use super::super::callbacks_switch::{SwitchCallbacks, create_switch_callbacks};
use super::super::callbacks_sync::{SyncCallbackSignals, SyncCallbacks, create_sync_callbacks};
use super::super::state::CoreSignals;
use super::scope;

pub(super) fn build_doc_callbacks(ws: &WsService, signals: &CoreSignals) -> DocCallbacks {
    create_doc_callbacks(
        ws,
        DocCallbackSignals {
            current_doc: signals.current_doc,
            local_scope: scope::local_scope(signals),
            write_gate: scope::repo_write(signals),
            pending_local_edits: signals.pending_local_edits,
            set_pending_navigation: signals.set_pending_navigation,
            set_current_doc: signals.set_current_doc,
            set_sync_banner: signals.set_sync_banner,
            set_pending_created_doc_path: signals.set_pending_created_doc_path,
            set_explicit_home: signals.set_explicit_home,
        },
    )
}

pub(super) fn build_sync_callbacks(ws: &WsService, signals: &CoreSignals) -> SyncCallbacks {
    create_sync_callbacks(
        ws,
        SyncCallbackSignals {
            current_doc: signals.current_doc,
            local_scope: scope::local_scope(signals),
            write_gate: scope::repo_write(signals),
            set_sync_banner: signals.set_sync_banner,
            set_shadow_list_request_id: signals.set_shadow_list_request_id,
            set_sync_mode_request_id: signals.set_sync_mode_request_id,
            set_pending_ops_request_id: signals.set_pending_ops_request_id,
        },
    )
}

pub(super) fn build_source_control_callbacks(
    ws: &WsService,
    signals: &CoreSignals,
) -> SourceControlCallbacks {
    create_source_control_callbacks(
        ws,
        scope::source_control_scope(signals),
        scope::repo_write(signals),
        scope::source_control_requests(signals),
        signals.set_source_control_notice,
        signals.set_diff_content,
        signals.set_sync_banner,
    )
}

pub(super) fn build_misc_callbacks(ws: &WsService, signals: &CoreSignals) -> MiscCallbacks {
    create_misc_callbacks(
        ws,
        signals.set_stats,
        signals.load_state,
        scope::search_scope(signals),
        scope::misc_requests(signals),
        signals.set_sync_banner,
    )
}

pub(super) fn build_switch_callbacks(ws: &WsService, signals: &CoreSignals) -> SwitchCallbacks {
    create_switch_callbacks(ws, scope::switch_scope(signals), signals.set_sync_banner)
}
