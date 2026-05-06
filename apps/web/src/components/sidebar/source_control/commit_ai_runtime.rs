//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 10_ai_agent#native-ai-chat-runtime
//!
use crate::api::{BackendSendDecision, ai_backend_to_plugin_id};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum CommitAiBackendPlan {
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
pub(super) enum CommitAiRuntimePlan {
    Dispatch {
        plugin_id: &'static str,
        switch_backend: Option<&'static str>,
        notice: Option<String>,
    },
    Block {
        reason: String,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum CommitAiRuntimeEffect {
    SwitchBackend(&'static str),
    AppendNotice(String),
    RegisterActiveRequest,
    AppendPlaceholder,
    DispatchPlugin { plugin_id: &'static str },
    AppendBlockReason(String),
    StopStreaming,
    StopGenerating,
    ClearActiveRequest,
}

pub(super) trait CommitAiEffectRunner {
    fn switch_backend(&mut self, backend: &'static str);
    fn append_notice(&mut self, notice: String);
    fn register_active_request(&mut self);
    fn append_placeholder(&mut self);
    fn dispatch_plugin(&mut self, plugin_id: &'static str);
    fn append_block_reason(&mut self, reason: String);
    fn stop_streaming(&mut self);
    fn stop_generating(&mut self);
    fn clear_active_request(&mut self);
}

pub(super) fn plan_commit_ai_backend_call(decision: BackendSendDecision) -> CommitAiBackendPlan {
    match decision {
        BackendSendDecision::Use(backend) => CommitAiBackendPlan::Call {
            plugin_id: ai_backend_to_plugin_id(backend),
        },
        BackendSendDecision::Switch { backend, reason } => CommitAiBackendPlan::Switch {
            backend,
            plugin_id: ai_backend_to_plugin_id(backend),
            notice: reason,
        },
        BackendSendDecision::Block { reason } => CommitAiBackendPlan::Block { reason },
    }
}

pub(super) fn plan_commit_ai_runtime(decision: BackendSendDecision) -> CommitAiRuntimePlan {
    match plan_commit_ai_backend_call(decision) {
        CommitAiBackendPlan::Call { plugin_id } => CommitAiRuntimePlan::Dispatch {
            plugin_id,
            switch_backend: None,
            notice: None,
        },
        CommitAiBackendPlan::Switch {
            backend,
            plugin_id,
            notice,
        } => CommitAiRuntimePlan::Dispatch {
            plugin_id,
            switch_backend: Some(backend),
            notice: Some(notice),
        },
        CommitAiBackendPlan::Block { reason } => CommitAiRuntimePlan::Block { reason },
    }
}

pub(super) fn plan_commit_ai_effects(plan: &CommitAiRuntimePlan) -> Vec<CommitAiRuntimeEffect> {
    match plan {
        CommitAiRuntimePlan::Dispatch {
            plugin_id,
            switch_backend,
            notice,
        } => {
            let mut effects = Vec::new();
            if let Some(backend) = switch_backend {
                effects.push(CommitAiRuntimeEffect::SwitchBackend(backend));
            }
            if let Some(notice) = notice {
                effects.push(CommitAiRuntimeEffect::AppendNotice(notice.clone()));
            }
            effects.push(CommitAiRuntimeEffect::RegisterActiveRequest);
            effects.push(CommitAiRuntimeEffect::AppendPlaceholder);
            effects.push(CommitAiRuntimeEffect::DispatchPlugin { plugin_id });
            effects
        }
        CommitAiRuntimePlan::Block { reason } => vec![
            CommitAiRuntimeEffect::AppendBlockReason(reason.clone()),
            CommitAiRuntimeEffect::StopStreaming,
            CommitAiRuntimeEffect::StopGenerating,
            CommitAiRuntimeEffect::ClearActiveRequest,
        ],
    }
}

pub(super) fn run_commit_ai_effects(
    effects: Vec<CommitAiRuntimeEffect>,
    runner: &mut impl CommitAiEffectRunner,
) {
    for effect in effects {
        match effect {
            CommitAiRuntimeEffect::SwitchBackend(backend) => runner.switch_backend(backend),
            CommitAiRuntimeEffect::AppendNotice(notice) => runner.append_notice(notice),
            CommitAiRuntimeEffect::RegisterActiveRequest => runner.register_active_request(),
            CommitAiRuntimeEffect::AppendPlaceholder => runner.append_placeholder(),
            CommitAiRuntimeEffect::DispatchPlugin { plugin_id } => {
                runner.dispatch_plugin(plugin_id)
            }
            CommitAiRuntimeEffect::AppendBlockReason(reason) => runner.append_block_reason(reason),
            CommitAiRuntimeEffect::StopStreaming => runner.stop_streaming(),
            CommitAiRuntimeEffect::StopGenerating => runner.stop_generating(),
            CommitAiRuntimeEffect::ClearActiveRequest => runner.clear_active_request(),
        }
    }
}
