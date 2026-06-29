//! plan_ref:
//!   - 07_network#web-ws-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::sync_banner_notice::warn_sync_banner;
use crate::hooks::use_core::write_gate_banner::{WriteGateAction, WriteGateReason, cannot_action};
use crate::hooks::use_core::{LoadPhase, PendingBranchSwitch, PendingRepoSwitch, SearchHit};
use crate::i18n::Locale;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

pub struct MiscCallbacks {
    pub on_stats: Callback<crate::editor::EditorStats>,
    pub on_plugin_call: Callback<(String, String, String, Vec<serde_json::Value>)>,
    pub on_search: Callback<String>,
}

pub struct SearchScopeSignals {
    pub current_scope_nonce: ReadSignal<u64>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchSwitch>>,
    pub pending_repo_switch: ReadSignal<Option<PendingRepoSwitch>>,
}

pub struct MiscRequestSignals {
    pub set_plugin_request_ids: WriteSignal<Vec<String>>,
    pub set_search_request_id: WriteSignal<Option<String>>,
    pub set_search_results: WriteSignal<Vec<SearchHit>>,
}

pub fn create_misc_callbacks(
    ws: &WsService,
    locale: RwSignal<Locale>,
    set_stats: WriteSignal<crate::editor::EditorStats>,
    load_state: ReadSignal<LoadPhase>,
    search_scope: SearchScopeSignals,
    request_signals: MiscRequestSignals,
    set_sync_banner: WriteSignal<Option<String>>,
) -> MiscCallbacks {
    let on_stats = Callback::new(move |s: crate::editor::EditorStats| set_stats.set(s));
    let ws_plugin = ws.clone();
    let on_plugin_call = Callback::new(
        move |(req_id, plugin_id, fn_name, args): (
            String,
            String,
            String,
            Vec<serde_json::Value>,
        )| {
            request_signals.set_plugin_request_ids.update(|ids| {
                if !ids.iter().any(|id| id == &req_id) {
                    ids.push(req_id.clone());
                }
            });
            ws_plugin.send(ClientMessage::PluginCall {
                req_id,
                plugin_id,
                fn_name,
                args,
            });
        },
    );
    let ws_search = ws.clone();
    let on_search = Callback::new(move |query: String| {
        if !load_state.get_untracked().is_ready() {
            show_search_block(set_sync_banner, locale, WriteGateReason::SnapshotLoading);
            return;
        }
        if search_scope.pending_branch_switch.get_untracked().is_some()
            || search_scope.pending_repo_switch.get_untracked().is_some()
        {
            show_search_block(set_sync_banner, locale, WriteGateReason::ScopeSwitching);
            return;
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        request_signals.set_search_results.set(Vec::new());
        request_signals
            .set_search_request_id
            .set(Some(request_id.clone()));
        ws_search.send(ClientMessage::Search {
            request_id,
            query,
            limit: 50,
            scope_nonce: Some(search_scope.current_scope_nonce.get_untracked()),
        });
    });
    MiscCallbacks {
        on_stats,
        on_plugin_call,
        on_search,
    }
}

fn show_search_block(
    set_sync_banner: WriteSignal<Option<String>>,
    locale: RwSignal<Locale>,
    reason: WriteGateReason,
) {
    let message = cannot_action(locale.get_untracked(), WriteGateAction::Search, reason);
    warn_sync_banner(set_sync_banner, message);
}

#[cfg(test)]
mod tests;
