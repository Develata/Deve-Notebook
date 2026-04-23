use crate::editor::EditorStats;
use leptos::prelude::*;

use super::super::callbacks::MiscCallbacks;
use super::super::state::CoreSignals;
use super::super::state::PluginResponse;
use super::super::types::ChatMessage;

pub(super) struct RuntimeStateSection {
    pub status_text: Signal<String>,
    pub sync_banner: Signal<Option<String>>,
    pub set_sync_banner: WriteSignal<Option<String>>,
    pub stats: ReadSignal<EditorStats>,
    pub on_stats: Callback<EditorStats>,
    pub plugin_last_response: ReadSignal<PluginResponse>,
    pub plugin_request_ids: ReadSignal<Vec<String>>,
    pub on_plugin_call: Callback<(String, String, String, Vec<serde_json::Value>)>,
    pub search_results: ReadSignal<Vec<(String, String, f32)>>,
    pub on_search: Callback<String>,
    pub load_state: ReadSignal<String>,
    pub set_load_state: WriteSignal<String>,
    pub load_progress: ReadSignal<(usize, usize)>,
    pub set_load_progress: WriteSignal<(usize, usize)>,
    pub load_eta_ms: ReadSignal<u64>,
    pub set_load_eta_ms: WriteSignal<u64>,
    pub chat_messages: ReadSignal<Vec<ChatMessage>>,
    pub set_chat_messages: WriteSignal<Vec<ChatMessage>>,
    pub is_chat_streaming: ReadSignal<bool>,
    pub set_is_chat_streaming: WriteSignal<bool>,
    pub ai_mode: ReadSignal<String>,
    pub set_ai_mode: WriteSignal<String>,
}

pub(super) fn build_runtime_section(
    signals: &CoreSignals,
    status_text: Signal<String>,
    misc: &MiscCallbacks,
) -> RuntimeStateSection {
    RuntimeStateSection {
        status_text,
        sync_banner: signals.sync_banner.into(),
        set_sync_banner: signals.set_sync_banner,
        stats: signals.stats,
        on_stats: misc.on_stats,
        plugin_last_response: signals.plugin_response,
        plugin_request_ids: signals.plugin_request_ids,
        on_plugin_call: misc.on_plugin_call,
        search_results: signals.search_results,
        on_search: misc.on_search,
        load_state: signals.load_state,
        set_load_state: signals.set_load_state,
        load_progress: signals.load_progress,
        set_load_progress: signals.set_load_progress,
        load_eta_ms: signals.load_eta_ms,
        set_load_eta_ms: signals.set_load_eta_ms,
        chat_messages: signals.chat_messages,
        set_chat_messages: signals.set_chat_messages,
        is_chat_streaming: signals.is_chat_streaming,
        set_is_chat_streaming: signals.set_is_chat_streaming,
        ai_mode: signals.ai_mode,
        set_ai_mode: signals.set_ai_mode,
    }
}
