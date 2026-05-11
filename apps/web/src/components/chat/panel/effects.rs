//! plan_ref:
//!   - 10_ai_agent#native-ai-chat-runtime
//!   - 05_network#web-ws-runtime
//!
use crate::hooks::use_core::CoreState;
use deve_core::protocol::ServerErrorCode;
use leptos::prelude::*;

pub fn attach_scope_reset_effect(
    core: CoreState,
    set_pending_reqs: WriteSignal<Vec<String>>,
    set_error_code: WriteSignal<Option<ServerErrorCode>>,
    set_last_prompt: WriteSignal<String>,
) {
    Effect::new(move |_| {
        let _ = (
            core.current_repo.get(),
            core.active_branch.get(),
            core.pending_repo_switch.get(),
            core.pending_branch_switch.get(),
        );
        set_pending_reqs.set(Vec::new());
        set_error_code.set(None);
        set_last_prompt.set(String::new());
    });
}

pub fn attach_plugin_response_effect(
    core: CoreState,
    pending_reqs: ReadSignal<Vec<String>>,
    set_pending_reqs: WriteSignal<Vec<String>>,
    set_error_code: WriteSignal<Option<ServerErrorCode>>,
) {
    Effect::new(move |_| {
        let Some((req_id, _result, error)) = core.plugin_last_response.get() else {
            return;
        };
        let matched = pending_reqs.get_untracked().iter().any(|id| id == &req_id);
        if !matched {
            return;
        }
        set_pending_reqs.update(|v| v.retain(|id| id != &req_id));
        if let Some(err) = error {
            if let Some(detail) = err.detail.as_deref() {
                leptos::logging::warn!("Plugin request {} failed: {}", req_id, detail);
            }
            set_error_code.set(Some(err.code));
        }
    });
}
