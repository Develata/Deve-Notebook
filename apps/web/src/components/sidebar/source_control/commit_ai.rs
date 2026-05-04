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
struct CommitAiRuntimePlan {
    plugin_id: Option<&'static str>,
    switch_backend: Option<&'static str>,
    notice: Option<String>,
    block_reason: Option<String>,
    register_active_req: bool,
    append_placeholder: bool,
    stop_streaming: bool,
    stop_generating: bool,
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
            let CommitAiRuntimePlan {
                plugin_id,
                switch_backend,
                notice,
                block_reason,
                register_active_req,
                append_placeholder,
                stop_streaming,
                stop_generating,
            } = plan_commit_ai_runtime(decision);
            if let Some(backend) = switch_backend {
                chat_ctx_for_send.set_ai_mode.set(backend.to_string());
            }
            if let Some(notice) = notice {
                append_commit_ai_message(&chat_ctx_for_send, notice, None);
            }
            if let Some(reason) = block_reason {
                append_commit_ai_message(&chat_ctx_for_send, reason, None);
            }
            if stop_streaming {
                chat_ctx_for_send.set_is_streaming.set(false);
            }
            if stop_generating {
                set_is_generating.set(false);
            }
            let Some(plugin_id) = plugin_id else {
                active_req_id.set(None);
                return;
            };
            if register_active_req {
                active_req_id.set(Some(req_id.clone()));
            }
            if append_placeholder {
                append_commit_ai_message(&chat_ctx_for_send, String::new(), Some(req_id.clone()));
            }
            chat_ctx_for_send.on_plugin_call.run((
                req_id,
                plugin_id.to_string(),
                "chat".to_string(),
                args,
            ));
        });
    })
}

fn append_commit_ai_message(chat_ctx: &ChatContext, content: String, req_id: Option<String>) {
    chat_ctx.set_messages.update(|messages| {
        messages.push(ChatMessage {
            role: "assistant".into(),
            content,
            req_id,
            ts_ms: js_sys::Date::now() as u64,
        });
    });
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
        CommitAiBackendPlan::Call { plugin_id } => CommitAiRuntimePlan {
            plugin_id: Some(plugin_id),
            switch_backend: None,
            notice: None,
            block_reason: None,
            register_active_req: true,
            append_placeholder: true,
            stop_streaming: false,
            stop_generating: false,
        },
        CommitAiBackendPlan::Switch {
            backend,
            plugin_id,
            notice,
        } => CommitAiRuntimePlan {
            plugin_id: Some(plugin_id),
            switch_backend: Some(backend),
            notice: Some(notice),
            block_reason: None,
            register_active_req: true,
            append_placeholder: true,
            stop_streaming: false,
            stop_generating: false,
        },
        CommitAiBackendPlan::Block { reason } => CommitAiRuntimePlan {
            plugin_id: None,
            switch_backend: None,
            notice: None,
            block_reason: Some(reason),
            register_active_req: false,
            append_placeholder: false,
            stop_streaming: true,
            stop_generating: true,
        },
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
