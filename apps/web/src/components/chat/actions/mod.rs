//! plan_ref:
//!   - 16_ai_agent#native-ai-chat-runtime
//!
mod apply;
mod send;
mod send_backend;

pub use apply::ChatApplyRuntime;
pub use apply::make_on_apply;
pub use send::{
    ChatSendControls, ChatSendRuntime, make_send_example, make_send_message, make_send_text,
};
