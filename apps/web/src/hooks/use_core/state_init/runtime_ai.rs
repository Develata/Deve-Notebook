use leptos::prelude::*;

use super::super::super::state::PluginResponse;
use super::super::super::types::ChatMessage;

#[derive(Clone, Copy)]
pub(super) struct AiRuntimeSignals {
    pub plugin_response: ReadSignal<PluginResponse>,
    pub set_plugin_response: WriteSignal<PluginResponse>,
    pub plugin_request_ids: ReadSignal<Vec<String>>,
    pub set_plugin_request_ids: WriteSignal<Vec<String>>,
    pub chat_messages: ReadSignal<Vec<ChatMessage>>,
    pub set_chat_messages: WriteSignal<Vec<ChatMessage>>,
    pub is_chat_streaming: ReadSignal<bool>,
    pub set_is_chat_streaming: WriteSignal<bool>,
    pub ai_mode: ReadSignal<String>,
    pub set_ai_mode: WriteSignal<String>,
    pub search_request_id: ReadSignal<Option<String>>,
    pub set_search_request_id: WriteSignal<Option<String>>,
    pub search_results: ReadSignal<Vec<(String, String, f32)>>,
    pub set_search_results: WriteSignal<Vec<(String, String, f32)>>,
}

pub(super) fn init_ai_runtime_signals() -> AiRuntimeSignals {
    let (plugin_response, set_plugin_response) = signal(PluginResponse::default());
    let (plugin_request_ids, set_plugin_request_ids) = signal(Vec::<String>::new());
    let (chat_messages, set_chat_messages) = signal(Vec::<ChatMessage>::new());
    let (is_chat_streaming, set_is_chat_streaming) = signal(false);
    let (ai_mode, set_ai_mode) = signal("ai-chat".to_string());
    let (search_request_id, set_search_request_id) = signal(None::<String>);
    let (search_results, set_search_results) = signal(Vec::<(String, String, f32)>::new());

    AiRuntimeSignals {
        plugin_response,
        set_plugin_response,
        plugin_request_ids,
        set_plugin_request_ids,
        chat_messages,
        set_chat_messages,
        is_chat_streaming,
        set_is_chat_streaming,
        ai_mode,
        set_ai_mode,
        search_request_id,
        set_search_request_id,
        search_results,
        set_search_results,
    }
}

#[cfg(test)]
mod tests {
    use super::init_ai_runtime_signals;
    use leptos::prelude::GetUntracked;

    #[test]
    fn defaults_to_native_ai_chat_mode() {
        let signals = init_ai_runtime_signals();
        assert_eq!(signals.ai_mode.get_untracked(), "ai-chat");
    }
}
