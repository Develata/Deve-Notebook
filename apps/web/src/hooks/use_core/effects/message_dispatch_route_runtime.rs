//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::sync_banner_notice::show_temporary_sync_banner;
use crate::i18n::Locale;
use crate::runtime::domain::SearchHit;
use crate::runtime::remote_import_client::RemoteImportClient;
use deve_core::protocol::ServerMessage;
use leptos::prelude::GetUntracked;

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
    remote_import: &RemoteImportClient,
) -> Option<ServerMessage> {
    let msg = route_search_results_message(msg, signals)?;
    match msg {
        ServerMessage::RemoteImport(response) => {
            remote_import.accept(response);
            None
        }
        ServerMessage::RemoteProjectionPush(response) => {
            if accepts_remote_projection_push_response(&response, signals) {
                let message = response.error.map_or_else(
                    || {
                        crate::i18n::t::command_palette::remote_projection_push_succeeded(locale)
                            .to_string()
                    },
                    |error| crate::i18n::server_error::message(locale, error.code).to_string(),
                );
                show_temporary_sync_banner(signals.sync_banner, signals.set_sync_banner, message);
            }
            None
        }
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

fn accepts_remote_projection_push_response(
    response: &deve_core::protocol::RemoteProjectionPushResponse,
    signals: CoreSignals,
) -> bool {
    signals
        .current_repo_id
        .get_untracked()
        .and_then(|repo_id| repo_id.parse::<deve_core::models::RepoId>().ok())
        .is_some_and(|repo_id| repo_id == response.repo_id)
        && signals.active_branch.get_untracked() == response.branch
        && signals.current_scope_nonce.get_untracked() == response.scope_nonce.get()
        && signals.pending_repo_switch.get_untracked().is_none()
        && signals.pending_branch_switch.get_untracked().is_none()
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
    use deve_core::protocol::{
        RemoteProjectionPushResponse, ScopeNonce, ServerError, ServerErrorCode,
    };
    use leptos::prelude::*;
    use leptos::reactive::owner::Owner;

    fn init_runtime() -> (Owner, CoreSignals) {
        let runtime = Owner::new();
        runtime.set();
        let (connection_status, _) = signal(ConnectionStatus::Connected);
        (runtime, init_signals(connection_status))
    }

    fn remote_import_client(ws: &WsService, signals: CoreSignals) -> RemoteImportClient {
        RemoteImportClient::new(
            ws.clone(),
            signals.current_repo_id,
            signals.active_branch,
            signals.current_scope_nonce,
            signals.pending_branch_switch,
            signals.pending_repo_switch,
        )
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

    #[test]
    fn remote_projection_push_response_is_scope_filtered_and_uses_typed_error_only() {
        let (_runtime, signals) = init_runtime();
        let repo_id = RepoId::new_v4();
        signals.set_current_repo_id.set(Some(repo_id.to_string()));
        signals.set_current_scope_nonce.set(7);

        let ws = crate::api::WsService::new_for_test(crate::api::ConnectionStatus::Connected);
        let remote_import = remote_import_client(&ws, signals);
        let stale = route_runtime_message(
            ServerMessage::RemoteProjectionPush(RemoteProjectionPushResponse {
                request_id: uuid::Uuid::new_v4(),
                repo_id,
                branch: None,
                scope_nonce: ScopeNonce::new(6),
                error: Some(ServerError::with_detail(
                    ServerErrorCode::RemoteProjectionPushFailed,
                    "CANARY_PRIVATE_BACKEND_DETAIL",
                )),
            }),
            &ws,
            Locale::En,
            signals,
            &remote_import,
        );
        assert!(stale.is_none());
        assert!(signals.sync_banner.get_untracked().is_none());

        for code in [
            ServerErrorCode::RemoteProjectionLocatorInvalid,
            ServerErrorCode::RemoteProjectionProviderUnavailable,
            ServerErrorCode::RemoteProjectionPushFailed,
        ] {
            route_runtime_message(
                ServerMessage::RemoteProjectionPush(RemoteProjectionPushResponse {
                    request_id: uuid::Uuid::new_v4(),
                    repo_id,
                    branch: None,
                    scope_nonce: ScopeNonce::new(7),
                    error: Some(ServerError::with_detail(
                        code,
                        "CANARY_PRIVATE_BACKEND_DETAIL",
                    )),
                }),
                &crate::api::WsService::new_for_test(crate::api::ConnectionStatus::Connected),
                Locale::En,
                signals,
                &remote_import,
            );

            let banner = signals.sync_banner.get_untracked().expect("typed banner");
            assert!(!banner.contains("CANARY_PRIVATE_BACKEND_DETAIL"));
            assert_eq!(banner, crate::i18n::server_error::message(Locale::En, code));
        }
    }
}
