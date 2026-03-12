use leptos::prelude::*;

use super::super::state::CoreSignals;
use super::super::types::ChatMessage;

pub fn accepts_unscoped_update(signals: CoreSignals) -> bool {
    signals.pending_branch_switch.get_untracked().is_none()
        && signals.pending_repo_switch.get_untracked().is_none()
}

pub fn accepts_plugin_response(req_id: &str, signals: CoreSignals) -> bool {
    accepts_unscoped_update(signals)
        && signals
            .plugin_request_ids
            .get_untracked()
            .iter()
            .any(|id| id == req_id)
}

pub fn accepts_chat_chunk(req_id: &str, signals: CoreSignals) -> bool {
    accepts_unscoped_update(signals)
        && (signals
            .plugin_request_ids
            .get_untracked()
            .iter()
            .any(|id| id == req_id)
            || contains_chat_message(req_id, signals.chat_messages.get_untracked()))
}

pub fn accepts_search_results(request_id: &str, signals: CoreSignals) -> bool {
    accepts_unscoped_update(signals)
        && signals.search_request_id.get_untracked().as_deref() == Some(request_id)
}

fn contains_chat_message(req_id: &str, messages: Vec<ChatMessage>) -> bool {
    messages
        .iter()
        .any(|message| message.req_id.as_deref() == Some(req_id))
}

#[cfg(test)]
mod tests {
    use super::{
        accepts_chat_chunk, accepts_plugin_response, accepts_search_results,
        accepts_unscoped_update,
    };
    use crate::api::ConnectionStatus;
    use crate::hooks::use_core::PendingBranchTarget;
    use crate::hooks::use_core::state::init_signals;
    use crate::hooks::use_core::types::ChatMessage;
    use leptos::prelude::*;

    #[test]
    fn rejects_unscoped_updates_while_repo_switch_pending() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (connection_status, _) = signal(ConnectionStatus::Connected);
        let signals = init_signals(connection_status);
        signals.set_pending_repo_switch.set(Some("test".into()));
        assert!(!accepts_unscoped_update(signals));
    }

    #[test]
    fn rejects_unscoped_updates_while_branch_switch_pending() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (connection_status, _) = signal(ConnectionStatus::Connected);
        let signals = init_signals(connection_status);
        signals
            .set_pending_branch_switch
            .set(Some(PendingBranchTarget::Local));
        assert!(!accepts_unscoped_update(signals));
    }

    #[test]
    fn rejects_search_results_when_request_id_is_stale() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (connection_status, _) = signal(ConnectionStatus::Connected);
        let signals = init_signals(connection_status);
        signals.set_search_request_id.set(Some("fresh".into()));
        assert!(!accepts_search_results("stale", signals));
        assert!(accepts_search_results("fresh", signals));
    }

    #[test]
    fn rejects_plugin_response_when_req_id_is_stale() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (connection_status, _) = signal(ConnectionStatus::Connected);
        let signals = init_signals(connection_status);
        signals.set_plugin_request_ids.set(vec!["fresh".into()]);
        assert!(!accepts_plugin_response("stale", signals));
        assert!(accepts_plugin_response("fresh", signals));
    }

    #[test]
    fn accepts_chat_finish_for_existing_message_after_response_ack() {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (connection_status, _) = signal(ConnectionStatus::Connected);
        let signals = init_signals(connection_status);
        signals.set_chat_messages.set(vec![ChatMessage {
            role: "assistant".into(),
            content: String::new(),
            req_id: Some("req-1".into()),
            ts_ms: 0,
        }]);
        assert!(accepts_chat_chunk("req-1", signals));
        assert!(!accepts_chat_chunk("stale", signals));
    }
}
