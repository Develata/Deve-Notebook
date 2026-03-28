use leptos::prelude::*;

use super::super::state::PluginResponse;
use super::super::types::ChatMessage;

#[derive(Clone)]
pub struct ChatContext {
    pub messages: ReadSignal<Vec<ChatMessage>>,
    pub set_messages: WriteSignal<Vec<ChatMessage>>,
    pub is_streaming: ReadSignal<bool>,
    pub set_is_streaming: WriteSignal<bool>,
    pub ai_mode: ReadSignal<String>,
    pub set_ai_mode: WriteSignal<String>,
    pub plugin_last_response: ReadSignal<PluginResponse>,
    pub on_plugin_call: Callback<(String, String, String, Vec<serde_json::Value>)>,
}
