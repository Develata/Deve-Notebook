use crate::api::WsService;

use super::super::callbacks::{
    DocCallbacks, MiscCallbacks, SourceControlCallbacks, SwitchCallbacks, SyncCallbacks,
    create_doc_callbacks, create_misc_callbacks, create_source_control_callbacks,
    create_switch_callbacks, create_sync_callbacks,
};
use super::super::state::CoreSignals;
use super::scope;

pub(super) fn build_doc_callbacks(ws: &WsService, signals: &CoreSignals) -> DocCallbacks {
    create_doc_callbacks(
        ws,
        signals.current_doc,
        scope::local_scope(signals),
        scope::repo_write(signals),
        signals.pending_local_edits,
        signals.set_pending_navigation,
        signals.set_current_doc,
        signals.set_sync_banner,
        signals.set_pending_created_doc_path,
        signals.set_explicit_home,
    )
}

pub(super) fn build_sync_callbacks(ws: &WsService, signals: &CoreSignals) -> SyncCallbacks {
    create_sync_callbacks(
        ws,
        signals.current_doc,
        scope::local_scope(signals),
        scope::repo_write(signals),
        signals.set_sync_banner,
        signals.set_shadow_list_request_id,
        signals.set_sync_mode_request_id,
        signals.set_pending_ops_request_id,
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
    )
}

pub(super) fn build_switch_callbacks(ws: &WsService, signals: &CoreSignals) -> SwitchCallbacks {
    create_switch_callbacks(ws, scope::switch_scope(signals))
}
