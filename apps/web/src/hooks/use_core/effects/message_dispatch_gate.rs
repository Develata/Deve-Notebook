use leptos::prelude::GetUntracked;

use super::super::state::CoreSignals;
#[path = "message_dispatch_gate_logic.rs"]
mod logic;

pub fn accepts_unscoped_update(signals: CoreSignals) -> bool {
    logic::accepts_unscoped_update(
        signals.pending_branch_switch.get_untracked(),
        signals.pending_repo_switch.get_untracked(),
    )
}

pub fn accepts_plugin_response(req_id: &str, signals: CoreSignals) -> bool {
    accepts_unscoped_update(signals)
        && logic::contains_request_id(req_id, &signals.plugin_request_ids.get_untracked())
}

pub fn accepts_chat_chunk(req_id: &str, signals: CoreSignals) -> bool {
    accepts_unscoped_update(signals)
        && (logic::contains_request_id(req_id, &signals.plugin_request_ids.get_untracked())
            || logic::contains_chat_message(req_id, &signals.chat_messages.get_untracked()))
}

pub fn accepts_search_results(
    request_id: &str,
    scope_nonce: Option<u64>,
    signals: CoreSignals,
) -> bool {
    accepts_unscoped_update(signals)
        && scope_nonce == Some(signals.current_scope_nonce.get_untracked())
        && signals.search_request_id.get_untracked().as_deref() == Some(request_id)
}

#[cfg(test)]
#[path = "message_dispatch_gate_test.rs"]
mod tests;
