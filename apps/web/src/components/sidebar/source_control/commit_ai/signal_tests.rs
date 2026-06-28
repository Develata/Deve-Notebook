use super::super::commit_ai_runtime::{
    CommitAiRuntimeEffect, plan_commit_ai_effects, plan_commit_ai_runtime, run_commit_ai_effects,
};
use super::CommitAiSignalEffectRunner;
use crate::api::{AI_PLUGIN_TRUSTED_CLI, BackendSendDecision};
use crate::hooks::use_core::state::PluginResponse;
use crate::hooks::use_core::{AiBackendMode, ChatContext, ChatMessage};
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use leptos::reactive::owner::Owner;

type PluginCall = (String, String, String, Vec<serde_json::Value>);

#[test]
fn source_control_commit_ai_signal_runner_dispatches_full_plugin_call_tuple() {
    let _runtime = Owner::new();
    _runtime.set();
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (is_streaming, set_is_streaming) = signal(true);
    let (ai_mode, set_ai_mode) = signal(AiBackendMode::Native);
    let (plugin_last_response, _) = signal(PluginResponse::default());
    let (plugin_calls, set_plugin_calls) = signal(Vec::<PluginCall>::new());
    let active_req_id = RwSignal::new(None::<String>);
    let (_, set_is_generating) = signal(false);
    let on_plugin_call = Callback::new(move |call: PluginCall| {
        set_plugin_calls.update(|calls| calls.push(call));
    });
    let chat_ctx = ChatContext {
        messages,
        set_messages,
        is_streaming,
        set_is_streaming,
        ai_mode,
        set_ai_mode,
        plugin_last_response,
        on_plugin_call,
    };
    let args = vec![
        serde_json::json!("req-42"),
        serde_json::json!("commit prompt"),
        serde_json::json!(""),
    ];
    let mut runner = CommitAiSignalEffectRunner {
        chat_ctx,
        locale: RwSignal::new(Locale::En),
        active_req_id,
        set_is_generating,
        req_id: "req-42".to_string(),
        args: args.clone(),
    };

    run_commit_ai_effects(
        vec![
            CommitAiRuntimeEffect::RegisterActiveRequest,
            CommitAiRuntimeEffect::AppendPlaceholder,
            CommitAiRuntimeEffect::DispatchPlugin {
                plugin_id: AI_PLUGIN_TRUSTED_CLI,
            },
        ],
        &mut runner,
    );

    assert_eq!(active_req_id.get_untracked().as_deref(), Some("req-42"));
    let messages = messages.get_untracked();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].role, "assistant");
    assert_eq!(messages[0].content, "");
    assert_eq!(messages[0].req_id.as_deref(), Some("req-42"));
    assert_eq!(
        plugin_calls.get_untracked(),
        vec![(
            "req-42".to_string(),
            AI_PLUGIN_TRUSTED_CLI.to_string(),
            "chat".to_string(),
            args
        )]
    );
}

#[test]
fn source_control_commit_ai_signal_runner_block_stops_active_request() {
    let _runtime = Owner::new();
    _runtime.set();
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (is_streaming, set_is_streaming) = signal(true);
    let (ai_mode, set_ai_mode) = signal(AiBackendMode::Native);
    let (plugin_last_response, _) = signal(PluginResponse::default());
    let (plugin_calls, set_plugin_calls) = signal(Vec::<PluginCall>::new());
    let active_req_id = RwSignal::new(Some("req-42".to_string()));
    let (is_generating, set_is_generating) = signal(true);
    let on_plugin_call = Callback::new(move |call: PluginCall| {
        set_plugin_calls.update(|calls| calls.push(call));
    });
    let chat_ctx = ChatContext {
        messages,
        set_messages,
        is_streaming,
        set_is_streaming,
        ai_mode,
        set_ai_mode,
        plugin_last_response,
        on_plugin_call,
    };
    let mut runner = CommitAiSignalEffectRunner {
        chat_ctx,
        locale: RwSignal::new(Locale::Zh),
        active_req_id,
        set_is_generating,
        req_id: "req-42".to_string(),
        args: Vec::new(),
    };
    let plan = plan_commit_ai_runtime(BackendSendDecision::Block {
        reason: "native AI disabled by config".to_string(),
    });

    run_commit_ai_effects(plan_commit_ai_effects(&plan), &mut runner);

    let messages = messages.get_untracked();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].role, "assistant");
    assert_eq!(
        messages[0].content,
        t::extensions::ai_backend_reason(Locale::Zh, "native AI disabled by config")
    );
    assert_eq!(messages[0].req_id, None);
    assert!(!is_streaming.get_untracked());
    assert!(!is_generating.get_untracked());
    assert_eq!(active_req_id.get_untracked(), None);
    assert!(plugin_calls.get_untracked().is_empty());
}

#[test]
fn source_control_commit_ai_signal_runner_localizes_backend_switch_notice() {
    let _runtime = Owner::new();
    _runtime.set();
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (is_streaming, set_is_streaming) = signal(true);
    let (ai_mode, set_ai_mode) = signal(AiBackendMode::Native);
    let (plugin_last_response, _) = signal(PluginResponse::default());
    let (_, set_plugin_calls) = signal(Vec::<PluginCall>::new());
    let active_req_id = RwSignal::new(None::<String>);
    let (_, set_is_generating) = signal(false);
    let on_plugin_call = Callback::new(move |call: PluginCall| {
        set_plugin_calls.update(|calls| calls.push(call));
    });
    let chat_ctx = ChatContext {
        messages,
        set_messages,
        is_streaming,
        set_is_streaming,
        ai_mode,
        set_ai_mode,
        plugin_last_response,
        on_plugin_call,
    };
    let mut runner = CommitAiSignalEffectRunner {
        chat_ctx,
        locale: RwSignal::new(Locale::Zh),
        active_req_id,
        set_is_generating,
        req_id: "req-42".to_string(),
        args: Vec::new(),
    };

    run_commit_ai_effects(
        vec![CommitAiRuntimeEffect::AppendNotice(
            "trusted mode required".to_string(),
        )],
        &mut runner,
    );

    let messages = messages.get_untracked();
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        t::extensions::ai_backend_fallback(Locale::Zh, "trusted mode required")
    );
    assert_ne!(messages[0].content, "trusted mode required");
}
