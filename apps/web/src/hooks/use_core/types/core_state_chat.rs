//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!
use super::core_state::CoreState;
use crate::runtime::domain::ChatMessage;
use leptos::prelude::Update;

impl CoreState {
    pub fn append_chat_message(&self, role: &str, content: &str, req_id: Option<String>) {
        self.set_chat_messages
            .update(|msgs: &mut Vec<ChatMessage>| {
                msgs.push(ChatMessage::new(
                    role,
                    content,
                    req_id,
                    js_sys::Date::now() as u64,
                ));
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
                    msg.append_content(delta);
                    return;
                }

                msgs.push(ChatMessage::new(
                    "assistant",
                    delta,
                    Some(req_id.to_string()),
                    js_sys::Date::now() as u64,
                ));
            });
    }
}
