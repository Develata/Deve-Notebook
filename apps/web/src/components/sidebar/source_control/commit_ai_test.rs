use super::{
    CommitAiBackendPlan, CommitAiRuntimeEffect, CommitAiRuntimePlan, plan_commit_ai_backend_call,
    plan_commit_ai_effects, plan_commit_ai_runtime,
};
use crate::api::{
    AI_BACKEND_NATIVE, AI_BACKEND_TRUSTED_CLI, AI_PLUGIN_NATIVE, AI_PLUGIN_TRUSTED_CLI,
    BackendSendDecision,
};

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
