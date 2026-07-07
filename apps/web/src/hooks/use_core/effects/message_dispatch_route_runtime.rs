//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::i18n::Locale;
use crate::runtime::domain::SearchHit;
use deve_core::protocol::ServerMessage;

use super::super::state::CoreSignals;
use super::message_dispatch_gate::accepts_search_results;
use super::message_dispatch_runtime::{
    handle_chat_chunk_message, handle_plugin_response_message, handle_search_results_message,
};

pub fn route_runtime_message(
    msg: ServerMessage,
    _ws: &WsService,
    locale: Locale,
    signals: CoreSignals,
) -> Option<ServerMessage> {
    let msg = route_search_results_message(msg, signals)?;
    match msg {
        ServerMessage::PluginResponse {
            req_id,
            result,
            error,
        } => {
            handle_plugin_response_message(req_id, result, error, locale, signals);
            None
        }
        ServerMessage::ChatChunk {
            req_id,
            delta,
            finish_reason,
        } => {
            handle_chat_chunk_message(req_id, delta, finish_reason, signals);
            None
        }
        other => Some(other),
    }
}

fn route_search_results_message(msg: ServerMessage, signals: CoreSignals) -> Option<ServerMessage> {
    match msg {
        ServerMessage::SearchResults {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            results,
        } => {
            if !accepts_search_results(
                &request_id,
                repo_id.clone(),
                branch.clone(),
                scope_nonce,
                signals,
            ) {
                return None;
            }
            let results = results.into_iter().map(SearchHit::from).collect();
            handle_search_results_message(
                request_id,
                repo_id,
                branch,
                scope_nonce,
                results,
                signals,
            );
            None
        }
        other => Some(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ConnectionStatus;
    use crate::hooks::use_core::state::{CoreSignals, init_signals};
    use deve_core::models::RepoId;
    use leptos::prelude::*;
    use leptos::reactive::owner::Owner;

    fn init_runtime() -> (Owner, CoreSignals) {
        let runtime = Owner::new();
        runtime.set();
        let (connection_status, _) = signal(ConnectionStatus::Connected);
        (runtime, init_signals(connection_status))
    }

    #[test]
    fn route_search_results_rejects_stale_scope_before_state_update() {
        let (_runtime, signals) = init_runtime();
        let repo_id = RepoId::new_v4();
        signals.set_current_repo_id.set(Some(repo_id.to_string()));
        signals.set_current_scope_nonce.set(7);
        signals.set_search_request_id.set(Some("search-1".into()));

        let routed = route_search_results_message(
            ServerMessage::SearchResults {
                request_id: "search-1".into(),
                repo_id: Some(repo_id),
                branch: None,
                scope_nonce: Some(6),
                results: vec![("doc-1".into(), "notes/a.md".into(), 1.0)],
            },
            signals,
        );

        assert!(routed.is_none());
        assert!(signals.search_results.get_untracked().is_empty());
        assert_eq!(
            signals.search_request_id.get_untracked().as_deref(),
            Some("search-1")
        );
    }
}
