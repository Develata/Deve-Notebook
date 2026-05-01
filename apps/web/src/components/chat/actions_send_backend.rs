//! plan_ref:
//!   - 10_ai_agent#native-ai-chat-runtime
//!
use crate::api::{BackendSendDecision, ai_backend_to_plugin_id};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ChatBackendSendPlan {
    Call {
        plugin_id: &'static str,
    },
    Switch {
        backend: &'static str,
        plugin_id: &'static str,
        notice: String,
    },
    Block {
        reason: String,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ChatMessagePlan {
    UserInput,
    AssistantNotice(String),
    AssistantPlaceholder,
    AssistantError(String),
}

pub(super) fn plan_chat_backend_send(decision: BackendSendDecision) -> ChatBackendSendPlan {
    match decision {
        BackendSendDecision::Use(backend) => ChatBackendSendPlan::Call {
            plugin_id: ai_backend_to_plugin_id(backend),
        },
        BackendSendDecision::Switch { backend, reason } => ChatBackendSendPlan::Switch {
            backend,
            plugin_id: ai_backend_to_plugin_id(backend),
            notice: reason,
        },
        BackendSendDecision::Block { reason } => ChatBackendSendPlan::Block { reason },
    }
}

pub(super) fn plan_chat_messages(plan: &ChatBackendSendPlan) -> Vec<ChatMessagePlan> {
    match plan {
        ChatBackendSendPlan::Call { .. } => {
            vec![
                ChatMessagePlan::UserInput,
                ChatMessagePlan::AssistantPlaceholder,
            ]
        }
        ChatBackendSendPlan::Switch { notice, .. } => {
            vec![
                ChatMessagePlan::UserInput,
                ChatMessagePlan::AssistantNotice(notice.clone()),
                ChatMessagePlan::AssistantPlaceholder,
            ]
        }
        ChatBackendSendPlan::Block { reason } => {
            vec![
                ChatMessagePlan::UserInput,
                ChatMessagePlan::AssistantError(reason.clone()),
            ]
        }
    }
}

#[cfg(test)]
#[path = "actions_send_backend_test.rs"]
mod tests;
