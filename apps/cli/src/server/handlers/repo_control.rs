//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 07_network#repo-control-wire-contract
//!   - 09_web_thin_client_ledger#repo-control-client-contract
//!
//! Thin typed transport adapter for host-local aliases and owned repository
//! lifecycle jobs. Durable work remains owned by backend runtimes after this
//! handler returns or the connection disappears.

mod alias;
pub(crate) mod http;
mod lifecycle;
mod removal;
mod settlement;

pub(crate) use settlement::apply_lifecycle_settlement;

use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::{
    RepoControlRequest, RepoControlResponse, ServerError, ServerErrorCode, ServerMessage,
};
use std::sync::Arc;
use uuid::Uuid;

pub(crate) async fn handle_repo_control(
    state: &Arc<AppState>,
    channel: &DualChannel,
    session: &mut WsSession,
    request: RepoControlRequest,
) {
    if !session.is_browser_session() {
        send_simple_error(
            channel,
            request.request_id(),
            ServerErrorCode::RepoLifecycleInvalidRequest,
        );
        return;
    }
    match request {
        RepoControlRequest::SetAlias {
            request_id,
            repo_id,
            alias,
            expected_alias_revision,
        } => {
            alias::handle_set_alias(
                state,
                channel,
                session,
                request_id,
                repo_id,
                &alias,
                expected_alias_revision,
            )
            .await;
        }
        RepoControlRequest::SubmitLifecycle {
            request_id,
            lifecycle_intent,
        } => {
            lifecycle::handle_submit_lifecycle(
                state,
                channel,
                session,
                request_id,
                lifecycle_intent,
            )
            .await;
        }
        RepoControlRequest::GetLifecycle { request_id } => {
            lifecycle::handle_get_lifecycle(state, channel, session, request_id).await;
        }
        RepoControlRequest::PrepareLocalRepoRemoval {
            request_id,
            repo_id,
            current_scope_nonce,
            fallback_repo_id,
        } => {
            removal::handle_prepare(
                state,
                channel,
                session,
                request_id,
                repo_id,
                current_scope_nonce,
                fallback_repo_id,
            )
            .await;
        }
        RepoControlRequest::ExecuteLocalRepoRemoval {
            request_id,
            preparation_id,
            confirmation_token,
            fallback_binding,
            current_scope_nonce,
            switch_nonce,
        } => {
            removal::handle_execute(
                state,
                channel,
                session,
                request_id,
                preparation_id,
                confirmation_token,
                fallback_binding,
                current_scope_nonce,
                switch_nonce,
            )
            .await;
        }
    }
}

fn send_error(
    channel: &DualChannel,
    request_id: Uuid,
    code: ServerErrorCode,
    message: &'static str,
    error: &dyn std::fmt::Display,
) {
    tracing::warn!(%request_id, %error, "{message}");
    send_simple_error(channel, request_id, code);
}

fn send_simple_error(channel: &DualChannel, request_id: Uuid, code: ServerErrorCode) {
    channel.unicast(ServerMessage::RepoControl(RepoControlResponse::Error {
        request_id,
        error: ServerError::new(code),
    }));
}
