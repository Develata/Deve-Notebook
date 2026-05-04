//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 10_ai_agent#native-ai-chat-runtime
//!
use crate::api::{
    BackendSendDecision, ai_backend_to_plugin_id, fetch_ai_backend_capabilities,
    resolve_backend_for_send,
};
use crate::hooks::use_core::{ChatContext, ChatMessage, SourceControlContext};
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[derive(Debug, PartialEq, Eq)]
enum CommitAiBackendPlan {
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
enum CommitAiRuntimePlan {
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
enum CommitAiRuntimeEffect {
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

trait CommitAiEffectRunner {
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

pub fn build_generate_callback(
    core: SourceControlContext,
    chat_ctx: ChatContext,
    locale: RwSignal<Locale>,
    active_req_id: RwSignal<Option<String>>,
    saw_streaming: RwSignal<bool>,
    set_is_generating: WriteSignal<bool>,
) -> Callback<()> {
    Callback::new(move |_| {
        if !core.can_write.get_untracked() || core.staged_changes.get_untracked().is_empty() {
            return;
        }
        let req_id = uuid::Uuid::new_v4().to_string();
        let joined_paths = core
            .staged_changes
            .get()
            .into_iter()
            .map(|entry| entry.path)
            .collect::<Vec<_>>()
            .join("\n");
        let prompt = format!(
            "{}\n{}",
            t::source_control::generate_prompt(locale.get()),
            joined_paths
        );
        let args = vec![
            serde_json::json!(req_id),
            serde_json::json!(prompt),
            serde_json::json!(""),
        ];
        saw_streaming.set(false);
        set_is_generating.set(true);
        chat_ctx.set_is_streaming.set(true);
        let chat_ctx_for_send = chat_ctx.clone();
        spawn_local(async move {
            let cap = fetch_ai_backend_capabilities().await;
            let decision =
                resolve_backend_for_send(chat_ctx_for_send.ai_mode.get_untracked().as_str(), &cap);
            let plan = plan_commit_ai_runtime(decision);
            let mut runner = CommitAiSignalEffectRunner {
                chat_ctx: chat_ctx_for_send,
                active_req_id,
                set_is_generating,
                req_id,
                args,
            };
            run_commit_ai_effects(plan_commit_ai_effects(&plan), &mut runner);
        });
    })
}

struct CommitAiSignalEffectRunner {
    chat_ctx: ChatContext,
    active_req_id: RwSignal<Option<String>>,
    set_is_generating: WriteSignal<bool>,
    req_id: String,
    args: Vec<serde_json::Value>,
}

impl CommitAiEffectRunner for CommitAiSignalEffectRunner {
    fn switch_backend(&mut self, backend: &'static str) {
        self.chat_ctx.set_ai_mode.set(backend.to_string());
    }

    fn append_notice(&mut self, notice: String) {
        append_commit_ai_message(&self.chat_ctx, notice, None);
    }

    fn register_active_request(&mut self) {
        self.active_req_id.set(Some(self.req_id.clone()));
    }

    fn append_placeholder(&mut self) {
        append_commit_ai_message(&self.chat_ctx, String::new(), Some(self.req_id.clone()));
    }

    fn dispatch_plugin(&mut self, plugin_id: &'static str) {
        self.chat_ctx.on_plugin_call.run((
            self.req_id.clone(),
            plugin_id.to_string(),
            "chat".to_string(),
            self.args.clone(),
        ));
    }

    fn append_block_reason(&mut self, reason: String) {
        append_commit_ai_message(&self.chat_ctx, reason, None);
    }

    fn stop_streaming(&mut self) {
        self.chat_ctx.set_is_streaming.set(false);
    }

    fn stop_generating(&mut self) {
        self.set_is_generating.set(false);
    }

    fn clear_active_request(&mut self) {
        self.active_req_id.set(None);
    }
}

fn append_commit_ai_message(chat_ctx: &ChatContext, content: String, req_id: Option<String>) {
    chat_ctx.set_messages.update(|messages| {
        messages.push(ChatMessage {
            role: "assistant".into(),
            content,
            req_id,
            ts_ms: current_ts_ms(),
        });
    });
}

#[cfg(target_arch = "wasm32")]
fn current_ts_ms() -> u64 {
    js_sys::Date::now() as u64
}

#[cfg(not(target_arch = "wasm32"))]
fn current_ts_ms() -> u64 {
    0
}

fn plan_commit_ai_backend_call(decision: BackendSendDecision) -> CommitAiBackendPlan {
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

fn plan_commit_ai_runtime(decision: BackendSendDecision) -> CommitAiRuntimePlan {
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

fn plan_commit_ai_effects(plan: &CommitAiRuntimePlan) -> Vec<CommitAiRuntimeEffect> {
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

fn run_commit_ai_effects(
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

pub fn sync_generated_commit_message(
    chat_ctx: ChatContext,
    active_req_id: RwSignal<Option<String>>,
    saw_streaming: RwSignal<bool>,
    set_msg: WriteSignal<String>,
    set_is_generating: WriteSignal<bool>,
) {
    Effect::new(move |_| {
        let req_id = active_req_id.get();
        let is_streaming = chat_ctx.is_streaming.get();
        if let Some(req_id) = req_id {
            if let Some(content) = chat_ctx
                .messages
                .get()
                .iter()
                .rev()
                .find(|m| m.req_id.as_deref() == Some(req_id.as_str()))
                .map(|m| m.content.clone())
            {
                set_msg.set(content);
            }
            if is_streaming {
                saw_streaming.set(true);
            }
            if saw_streaming.get_untracked() && !is_streaming {
                set_is_generating.set(false);
                saw_streaming.set(false);
                active_req_id.set(None);
            }
        }
    });
}

#[cfg(test)]
#[path = "commit_ai_test.rs"]
mod tests;
