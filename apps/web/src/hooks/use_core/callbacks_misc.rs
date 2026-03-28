use crate::api::WsService;
use crate::hooks::use_core::PendingBranchTarget;
use deve_core::protocol::ClientMessage;
use leptos::prelude::*;

pub struct MiscCallbacks {
    pub on_stats: Callback<crate::editor::EditorStats>,
    pub on_plugin_call: Callback<(String, String, String, Vec<serde_json::Value>)>,
    pub on_search: Callback<String>,
}

pub struct SearchScopeSignals {
    pub current_scope_nonce: ReadSignal<u64>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
}

pub struct MiscRequestSignals {
    pub set_plugin_request_ids: WriteSignal<Vec<String>>,
    pub set_search_request_id: WriteSignal<Option<String>>,
}

pub fn create_misc_callbacks(
    ws: &WsService,
    set_stats: WriteSignal<crate::editor::EditorStats>,
    load_state: ReadSignal<String>,
    search_scope: SearchScopeSignals,
    request_signals: MiscRequestSignals,
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
        if load_state.get_untracked() != "ready" {
            leptos::logging::warn!("Search disabled while loading");
            return;
        }
        if search_scope.pending_branch_switch.get_untracked().is_some()
            || search_scope.pending_repo_switch.get_untracked().is_some()
        {
            leptos::logging::warn!("Search disabled while scope switch is pending");
            return;
        }
        let request_id = uuid::Uuid::new_v4().to_string();
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
