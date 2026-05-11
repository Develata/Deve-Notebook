use super::super::contexts::ChatContext;
use super::super::types::CoreState;

pub(super) fn build_chat_context(state: &CoreState) -> ChatContext {
    ChatContext {
        messages: state.chat_messages,
        set_messages: state.set_chat_messages,
        is_streaming: state.is_chat_streaming,
        set_is_streaming: state.set_is_chat_streaming,
        ai_mode: state.ai_mode,
        set_ai_mode: state.set_ai_mode,
        plugin_last_response: state.plugin_last_response,
        on_plugin_call: state.on_plugin_call,
    }
}
