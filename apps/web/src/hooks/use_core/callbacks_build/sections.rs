//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::i18n::Locale;
use leptos::prelude::RwSignal;

use super::super::callbacks::{
    DocCallbackSignals, DocCallbacks, MiscCallbacks, create_doc_callbacks, create_misc_callbacks,
};
use super::super::callbacks_sc::{
    SourceControlCallbackInputs, SourceControlCallbacks, create_source_control_callbacks,
};
use super::super::callbacks_switch::{SwitchCallbacks, create_switch_callbacks};
use super::super::callbacks_sync::{SyncCallbackSignals, SyncCallbacks, create_sync_callbacks};
use super::super::state::CoreSignals;
use super::scope;

pub(super) fn build_doc_callbacks(
    ws: &WsService,
    signals: &CoreSignals,
    locale: RwSignal<Locale>,
) -> DocCallbacks {
    create_doc_callbacks(
        ws,
        DocCallbackSignals {
            locale,
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

pub(super) fn build_sync_callbacks(
    ws: &WsService,
    signals: &CoreSignals,
    locale: RwSignal<Locale>,
) -> SyncCallbacks {
    create_sync_callbacks(
        ws,
        SyncCallbackSignals {
            locale,
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
    locale: RwSignal<Locale>,
) -> SourceControlCallbacks {
    create_source_control_callbacks(
        ws,
        SourceControlCallbackInputs {
            locale,
            scope: scope::source_control_scope(signals),
            write_gate: scope::repo_write(signals),
            request: scope::source_control_requests(signals),
            set_notice: signals.set_source_control_notice,
            set_diff_content: signals.set_diff_content,
            set_sync_banner: signals.set_sync_banner,
        },
    )
}

pub(super) fn build_misc_callbacks(
    ws: &WsService,
    signals: &CoreSignals,
    locale: RwSignal<Locale>,
) -> MiscCallbacks {
    create_misc_callbacks(
        ws,
        locale,
        signals.set_stats,
        signals.load_state,
        scope::search_scope(signals),
        scope::misc_requests(signals),
        signals.set_sync_banner,
    )
}

pub(super) fn build_switch_callbacks(
    ws: &WsService,
    signals: &CoreSignals,
    locale: RwSignal<Locale>,
) -> SwitchCallbacks {
    create_switch_callbacks(
        ws,
        locale,
        scope::switch_scope(signals),
        signals.set_sync_banner,
    )
}
