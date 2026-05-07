use super::super::commit_ai_runtime::{CommitAiBackendPlan, plan_commit_ai_backend_call};
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
