use super::{
    CommitAiBackendPlan, CommitAiEffectRunner, CommitAiRuntimeEffect, CommitAiRuntimePlan,
    CommitAiSignalEffectRunner, plan_commit_ai_backend_call, plan_commit_ai_effects,
    plan_commit_ai_runtime, run_commit_ai_effects,
};
use crate::api::{
    AI_BACKEND_NATIVE, AI_BACKEND_TRUSTED_CLI, AI_PLUGIN_NATIVE, AI_PLUGIN_TRUSTED_CLI,
    BackendSendDecision,
};
use crate::hooks::use_core::state::PluginResponse;
use crate::hooks::use_core::{ChatContext, ChatMessage};
use leptos::prelude::*;
use leptos::reactive::owner::Owner;

type PluginCall = (String, String, String, Vec<serde_json::Value>);

#[test]
fn source_control_commit_ai_uses_native_backend_without_notice() {
    let plan = plan_commit_ai_backend_call(BackendSendDecision::Use(AI_BACKEND_NATIVE));

    assert_eq!(
        plan,
        CommitAiBackendPlan::Call {
            plugin_id: AI_PLUGIN_NATIVE
        }
    );
}

#[test]
fn source_control_commit_ai_switches_backend_with_visible_reason() {
    let plan = plan_commit_ai_backend_call(BackendSendDecision::Switch {
        backend: AI_BACKEND_NATIVE,
        reason: "trusted mode required".to_string(),
    });

    assert_eq!(
        plan,
        CommitAiBackendPlan::Switch {
            backend: AI_BACKEND_NATIVE,
            plugin_id: AI_PLUGIN_NATIVE,
            notice: "trusted mode required".to_string()
        }
    );
}

#[test]
fn source_control_commit_ai_switches_trusted_cli_to_agent_bridge() {
    let plan = plan_commit_ai_backend_call(BackendSendDecision::Switch {
        backend: AI_BACKEND_TRUSTED_CLI,
        reason: "trusted-cli explicitly requested".to_string(),
    });

    assert_eq!(
        plan,
        CommitAiBackendPlan::Switch {
            backend: AI_BACKEND_TRUSTED_CLI,
            plugin_id: AI_PLUGIN_TRUSTED_CLI,
            notice: "trusted-cli explicitly requested".to_string()
        }
    );
}

#[test]
fn source_control_commit_ai_blocks_without_plugin_call() {
    let plan = plan_commit_ai_backend_call(BackendSendDecision::Block {
        reason: "native AI disabled by config".to_string(),
    });

    assert_eq!(
        plan,
        CommitAiBackendPlan::Block {
            reason: "native AI disabled by config".to_string()
        }
    );
}

#[test]
fn source_control_commit_ai_maps_trusted_cli_to_agent_bridge() {
    let plan = plan_commit_ai_backend_call(BackendSendDecision::Use(AI_BACKEND_TRUSTED_CLI));

    assert_eq!(
        plan,
        CommitAiBackendPlan::Call {
            plugin_id: AI_PLUGIN_TRUSTED_CLI
        }
    );
}

#[test]
fn source_control_commit_ai_call_runtime_maps_native_dispatch() {
    let plan = plan_commit_ai_runtime(BackendSendDecision::Use(AI_BACKEND_NATIVE));

    assert_eq!(
        plan,
        CommitAiRuntimePlan::Dispatch {
            plugin_id: AI_PLUGIN_NATIVE,
            switch_backend: None,
            notice: None,
        }
    );
}

#[test]
fn source_control_commit_ai_trusted_cli_call_runtime_uses_agent_bridge() {
    let plan = plan_commit_ai_runtime(BackendSendDecision::Use(AI_BACKEND_TRUSTED_CLI));

    assert_eq!(
        plan,
        CommitAiRuntimePlan::Dispatch {
            plugin_id: AI_PLUGIN_TRUSTED_CLI,
            switch_backend: None,
            notice: None,
        }
    );
}

#[test]
fn source_control_commit_ai_switch_runtime_preserves_notice() {
    let plan = plan_commit_ai_runtime(BackendSendDecision::Switch {
        backend: AI_BACKEND_NATIVE,
        reason: "trusted mode required".to_string(),
    });

    assert_eq!(
        plan,
        CommitAiRuntimePlan::Dispatch {
            plugin_id: AI_PLUGIN_NATIVE,
            switch_backend: Some(AI_BACKEND_NATIVE),
            notice: Some("trusted mode required".to_string()),
        }
    );
}

#[test]
fn source_control_commit_ai_call_effects_register_placeholder_before_dispatch() {
    let plan = plan_commit_ai_runtime(BackendSendDecision::Use(AI_BACKEND_NATIVE));

    assert_eq!(
        plan_commit_ai_effects(&plan),
        vec![
            CommitAiRuntimeEffect::RegisterActiveRequest,
            CommitAiRuntimeEffect::AppendPlaceholder,
            CommitAiRuntimeEffect::DispatchPlugin {
                plugin_id: AI_PLUGIN_NATIVE,
            },
        ]
    );
}

#[test]
fn source_control_commit_ai_switch_effects_keep_notice_before_placeholder() {
    let plan = plan_commit_ai_runtime(BackendSendDecision::Switch {
        backend: AI_BACKEND_TRUSTED_CLI,
        reason: "trusted-cli explicitly requested".to_string(),
    });

    assert_eq!(
        plan_commit_ai_effects(&plan),
        vec![
            CommitAiRuntimeEffect::SwitchBackend(AI_BACKEND_TRUSTED_CLI),
            CommitAiRuntimeEffect::AppendNotice("trusted-cli explicitly requested".to_string()),
            CommitAiRuntimeEffect::RegisterActiveRequest,
            CommitAiRuntimeEffect::AppendPlaceholder,
            CommitAiRuntimeEffect::DispatchPlugin {
                plugin_id: AI_PLUGIN_TRUSTED_CLI,
            },
        ]
    );
}

#[test]
fn source_control_commit_ai_block_effects_stop_without_dispatch() {
    let plan = plan_commit_ai_runtime(BackendSendDecision::Block {
        reason: "native AI disabled by config".to_string(),
    });

    assert_eq!(
        plan_commit_ai_effects(&plan),
        vec![
            CommitAiRuntimeEffect::AppendBlockReason("native AI disabled by config".to_string()),
            CommitAiRuntimeEffect::StopStreaming,
            CommitAiRuntimeEffect::StopGenerating,
            CommitAiRuntimeEffect::ClearActiveRequest,
        ]
    );
}

#[test]
fn source_control_commit_ai_trusted_cli_switch_runtime_uses_agent_bridge() {
    let plan = plan_commit_ai_runtime(BackendSendDecision::Switch {
        backend: AI_BACKEND_TRUSTED_CLI,
        reason: "trusted-cli explicitly requested".to_string(),
    });

    assert_eq!(
        plan,
        CommitAiRuntimePlan::Dispatch {
            plugin_id: AI_PLUGIN_TRUSTED_CLI,
            switch_backend: Some(AI_BACKEND_TRUSTED_CLI),
            notice: Some("trusted-cli explicitly requested".to_string()),
        }
    );
}

#[test]
fn source_control_commit_ai_block_runtime_maps_block_reason() {
    let plan = plan_commit_ai_runtime(BackendSendDecision::Block {
        reason: "native AI disabled by config".to_string(),
    });

    assert_eq!(
        plan,
        CommitAiRuntimePlan::Block {
            reason: "native AI disabled by config".to_string(),
        }
    );
}

#[test]
fn source_control_commit_ai_effect_runner_dispatches_in_order() {
    let plan = plan_commit_ai_runtime(BackendSendDecision::Switch {
        backend: AI_BACKEND_TRUSTED_CLI,
        reason: "trusted-cli explicitly requested".to_string(),
    });
    let mut runner = RecordingCommitAiEffectRunner::default();

    run_commit_ai_effects(plan_commit_ai_effects(&plan), &mut runner);

    assert_eq!(
        runner.events,
        vec![
            CommitAiRunEvent::SwitchBackend(AI_BACKEND_TRUSTED_CLI),
            CommitAiRunEvent::AppendNotice("trusted-cli explicitly requested".to_string()),
            CommitAiRunEvent::RegisterActiveRequest,
            CommitAiRunEvent::AppendPlaceholder,
            CommitAiRunEvent::DispatchPlugin(AI_PLUGIN_TRUSTED_CLI),
        ]
    );
}

#[test]
fn source_control_commit_ai_effect_runner_blocks_in_order() {
    let plan = plan_commit_ai_runtime(BackendSendDecision::Block {
        reason: "native AI disabled by config".to_string(),
    });
    let mut runner = RecordingCommitAiEffectRunner::default();

    run_commit_ai_effects(plan_commit_ai_effects(&plan), &mut runner);

    assert_eq!(
        runner.events,
        vec![
            CommitAiRunEvent::AppendBlockReason("native AI disabled by config".to_string()),
            CommitAiRunEvent::StopStreaming,
            CommitAiRunEvent::StopGenerating,
            CommitAiRunEvent::ClearActiveRequest,
        ]
    );
}

#[test]
fn source_control_commit_ai_signal_runner_dispatches_full_plugin_call_tuple() {
    let _runtime = Owner::new();
    _runtime.set();
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (is_streaming, set_is_streaming) = signal(true);
    let (ai_mode, set_ai_mode) = signal(AI_BACKEND_NATIVE.to_string());
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

#[derive(Default)]
struct RecordingCommitAiEffectRunner {
    events: Vec<CommitAiRunEvent>,
}

#[derive(Debug, PartialEq, Eq)]
enum CommitAiRunEvent {
    SwitchBackend(&'static str),
    AppendNotice(String),
    RegisterActiveRequest,
    AppendPlaceholder,
    DispatchPlugin(&'static str),
    AppendBlockReason(String),
    StopStreaming,
    StopGenerating,
    ClearActiveRequest,
}

impl CommitAiEffectRunner for RecordingCommitAiEffectRunner {
    fn switch_backend(&mut self, backend: &'static str) {
        self.events.push(CommitAiRunEvent::SwitchBackend(backend));
    }

    fn append_notice(&mut self, notice: String) {
        self.events.push(CommitAiRunEvent::AppendNotice(notice));
    }

    fn register_active_request(&mut self) {
        self.events.push(CommitAiRunEvent::RegisterActiveRequest);
    }

    fn append_placeholder(&mut self) {
        self.events.push(CommitAiRunEvent::AppendPlaceholder);
    }

    fn dispatch_plugin(&mut self, plugin_id: &'static str) {
        self.events
            .push(CommitAiRunEvent::DispatchPlugin(plugin_id));
    }

    fn append_block_reason(&mut self, reason: String) {
        self.events
            .push(CommitAiRunEvent::AppendBlockReason(reason));
    }

    fn stop_streaming(&mut self) {
        self.events.push(CommitAiRunEvent::StopStreaming);
    }

    fn stop_generating(&mut self) {
        self.events.push(CommitAiRunEvent::StopGenerating);
    }

    fn clear_active_request(&mut self) {
        self.events.push(CommitAiRunEvent::ClearActiveRequest);
    }
}
