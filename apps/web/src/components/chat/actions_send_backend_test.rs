use super::{
    ChatBackendSendPlan, ChatMessagePlan, ChatSendRuntimePlan, plan_chat_backend_send,
    plan_chat_messages, plan_chat_plugin_id, plan_chat_send_runtime, plan_chat_switch_backend,
};
use crate::api::{
    AI_BACKEND_NATIVE, AI_BACKEND_TRUSTED_CLI, AI_PLUGIN_NATIVE, AI_PLUGIN_TRUSTED_CLI,
    BackendSendDecision,
};

#[test]
fn chat_send_uses_native_backend_without_notice() {
    let plan = plan_chat_backend_send(BackendSendDecision::Use(AI_BACKEND_NATIVE));

    assert_eq!(
        plan,
        ChatBackendSendPlan::Call {
            plugin_id: AI_PLUGIN_NATIVE
        }
    );
    assert_eq!(
        plan_chat_messages(&plan),
        vec![
            ChatMessagePlan::UserInput,
            ChatMessagePlan::AssistantPlaceholder,
        ]
    );
    assert_eq!(plan_chat_plugin_id(&plan), Some(AI_PLUGIN_NATIVE));
    assert_eq!(plan_chat_switch_backend(&plan), None);
}

#[test]
fn chat_send_switches_backend_after_user_message() {
    let plan = plan_chat_backend_send(BackendSendDecision::Switch {
        backend: AI_BACKEND_NATIVE,
        reason: "trusted mode required".to_string(),
    });

    assert_eq!(
        plan,
        ChatBackendSendPlan::Switch {
            backend: AI_BACKEND_NATIVE,
            plugin_id: AI_PLUGIN_NATIVE,
            notice: "trusted mode required".to_string()
        }
    );
    assert_eq!(
        plan_chat_messages(&plan),
        vec![
            ChatMessagePlan::UserInput,
            ChatMessagePlan::AssistantNotice("trusted mode required".to_string()),
            ChatMessagePlan::AssistantPlaceholder,
        ]
    );
    assert_eq!(plan_chat_plugin_id(&plan), Some(AI_PLUGIN_NATIVE));
    assert_eq!(plan_chat_switch_backend(&plan), Some(AI_BACKEND_NATIVE));
}

#[test]
fn trusted_cli_untrusted_send_uses_native_plugin_and_visible_notice() {
    let plan = plan_chat_backend_send(BackendSendDecision::Switch {
        backend: AI_BACKEND_NATIVE,
        reason: "trusted mode required".to_string(),
    });

    assert_eq!(
        plan,
        ChatBackendSendPlan::Switch {
            backend: AI_BACKEND_NATIVE,
            plugin_id: AI_PLUGIN_NATIVE,
            notice: "trusted mode required".to_string()
        }
    );
    assert_ne!(
        plan,
        ChatBackendSendPlan::Switch {
            backend: AI_BACKEND_NATIVE,
            plugin_id: AI_PLUGIN_TRUSTED_CLI,
            notice: "trusted mode required".to_string()
        }
    );
    assert_eq!(
        plan_chat_messages(&plan),
        vec![
            ChatMessagePlan::UserInput,
            ChatMessagePlan::AssistantNotice("trusted mode required".to_string()),
            ChatMessagePlan::AssistantPlaceholder,
        ]
    );
}

#[test]
fn chat_send_switches_trusted_cli_to_agent_bridge() {
    let plan = plan_chat_backend_send(BackendSendDecision::Switch {
        backend: AI_BACKEND_TRUSTED_CLI,
        reason: "trusted-cli explicitly requested".to_string(),
    });

    assert_eq!(
        plan,
        ChatBackendSendPlan::Switch {
            backend: AI_BACKEND_TRUSTED_CLI,
            plugin_id: AI_PLUGIN_TRUSTED_CLI,
            notice: "trusted-cli explicitly requested".to_string()
        }
    );
    assert_eq!(
        plan_chat_messages(&plan),
        vec![
            ChatMessagePlan::UserInput,
            ChatMessagePlan::AssistantNotice("trusted-cli explicitly requested".to_string()),
            ChatMessagePlan::AssistantPlaceholder,
        ]
    );
}

#[test]
fn chat_send_blocks_without_plugin_placeholder() {
    let plan = plan_chat_backend_send(BackendSendDecision::Block {
        reason: "native AI disabled by config".to_string(),
    });

    assert_eq!(
        plan,
        ChatBackendSendPlan::Block {
            reason: "native AI disabled by config".to_string()
        }
    );
    assert_eq!(
        plan_chat_messages(&plan),
        vec![
            ChatMessagePlan::UserInput,
            ChatMessagePlan::AssistantError("native AI disabled by config".to_string()),
        ]
    );
    assert_eq!(plan_chat_plugin_id(&plan), None);
    assert_eq!(plan_chat_switch_backend(&plan), None);
}

#[test]
fn chat_send_block_runtime_does_not_register_pending_request() {
    let runtime = plan_chat_send_runtime(BackendSendDecision::Block {
        reason: "native AI disabled by config".to_string(),
    });

    assert_eq!(
        runtime,
        ChatSendRuntimePlan {
            plugin_id: None,
            switch_backend: None,
            messages: vec![
                ChatMessagePlan::UserInput,
                ChatMessagePlan::AssistantError("native AI disabled by config".to_string()),
            ],
            register_pending_req: false,
            stop_streaming: true,
        }
    );
}

#[test]
fn chat_send_call_runtime_registers_pending_request() {
    let runtime = plan_chat_send_runtime(BackendSendDecision::Use(AI_BACKEND_NATIVE));

    assert_eq!(
        runtime,
        ChatSendRuntimePlan {
            plugin_id: Some(AI_PLUGIN_NATIVE),
            switch_backend: None,
            messages: vec![
                ChatMessagePlan::UserInput,
                ChatMessagePlan::AssistantPlaceholder,
            ],
            register_pending_req: true,
            stop_streaming: false,
        }
    );
}

#[test]
fn chat_send_maps_trusted_cli_to_agent_bridge() {
    let plan = plan_chat_backend_send(BackendSendDecision::Use(AI_BACKEND_TRUSTED_CLI));

    assert_eq!(
        plan,
        ChatBackendSendPlan::Call {
            plugin_id: AI_PLUGIN_TRUSTED_CLI
        }
    );
    assert_eq!(
        plan_chat_messages(&plan),
        vec![
            ChatMessagePlan::UserInput,
            ChatMessagePlan::AssistantPlaceholder,
        ]
    );
}
