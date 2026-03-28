use super::core_state::CoreState;
use super::shared::ChatMessage;
use leptos::prelude::Update;

impl CoreState {
    pub fn append_chat_message(&self, role: &str, content: &str, req_id: Option<String>) {
        self.set_chat_messages
            .update(|msgs: &mut Vec<ChatMessage>| {
                msgs.push(ChatMessage {
                    role: role.to_string(),
                    content: content.to_string(),
                    req_id,
                    ts_ms: js_sys::Date::now() as u64,
                });
            });
    }

    pub fn update_chat_message(&self, req_id: &str, delta: &str) {
        self.set_chat_messages
            .update(|msgs: &mut Vec<ChatMessage>| {
                if let Some(msg) = msgs
                    .iter_mut()
                    .rev()
                    .find(|m| m.req_id.as_deref() == Some(req_id))
                {
                    msg.content.push_str(delta);
                    return;
                }

                msgs.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: delta.to_string(),
                    req_id: Some(req_id.to_string()),
                    ts_ms: js_sys::Date::now() as u64,
                });
            });
    }
}
