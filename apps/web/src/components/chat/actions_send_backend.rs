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

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ChatSendRuntimePlan {
    pub(super) plugin_id: Option<&'static str>,
    pub(super) switch_backend: Option<&'static str>,
    pub(super) messages: Vec<ChatMessagePlan>,
    pub(super) register_pending_req: bool,
    pub(super) stop_streaming: bool,
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

pub(super) fn plan_chat_send_runtime(decision: BackendSendDecision) -> ChatSendRuntimePlan {
    let plan = plan_chat_backend_send(decision);
    let plugin_id = plan_chat_plugin_id(&plan);
    ChatSendRuntimePlan {
        plugin_id,
        switch_backend: plan_chat_switch_backend(&plan),
        messages: plan_chat_messages(&plan),
        register_pending_req: plugin_id.is_some(),
        stop_streaming: plugin_id.is_none(),
    }
}

pub(super) fn plan_chat_plugin_id(plan: &ChatBackendSendPlan) -> Option<&'static str> {
    match plan {
        ChatBackendSendPlan::Call { plugin_id } | ChatBackendSendPlan::Switch { plugin_id, .. } => {
            Some(*plugin_id)
        }
        ChatBackendSendPlan::Block { .. } => None,
    }
}

pub(super) fn plan_chat_switch_backend(plan: &ChatBackendSendPlan) -> Option<&'static str> {
    match plan {
        ChatBackendSendPlan::Switch { backend, .. } => Some(*backend),
        ChatBackendSendPlan::Call { .. } | ChatBackendSendPlan::Block { .. } => None,
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
