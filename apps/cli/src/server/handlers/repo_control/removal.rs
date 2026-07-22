//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!   - 07_network#repo-control-wire-contract
//!   - 09_web_thin_client_ledger#repo-control-client-contract
//!
//! Thin Web transport for backend-owned removal preparation and admission.

use super::{lifecycle, send_simple_error};
use crate::server::runtime::repo_lifecycle_job_runtime::{
    RepoRemovalExecuteIntent, RepoRemovalIssuerBinding, RepoRemovalPrepareIntent,
};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::models::RepoId;
use deve_core::protocol::{
    OpaqueFallbackBinding, RemovalConfirmationToken, RepoControlResponse, ScopeNonce,
    ServerErrorCode, ServerMessage, SwitchNonce,
};
use std::sync::Arc;
use uuid::Uuid;

pub(super) async fn handle_prepare(
    state: &Arc<AppState>,
    channel: &DualChannel,
    session: &WsSession,
    request_id: Uuid,
    repo_id: RepoId,
    current_scope_nonce: ScopeNonce,
    fallback_repo_id: Option<RepoId>,
) {
    let Some(issuer) = web_issuer(session) else {
        send_simple_error(
            channel,
            request_id,
            ServerErrorCode::RepoLifecycleInvalidRequest,
        );
        return;
    };
    // The scope nonce binds this browser observation, not the removal target.
    // A host-local repo may be removed from a different currently active local
    // scope; target identity is carried exclusively by the exact RepoId and
    // the backend manifest.
    if session.active_branch.is_some() || session.scope_nonce() != current_scope_nonce.get() {
        send_simple_error(
            channel,
            request_id,
            ServerErrorCode::RepoLifecycleInvalidRequest,
        );
        return;
    }
    let result = state
        .repo_lifecycle_jobs()
        .prepare_removal(RepoRemovalPrepareIntent {
            request_id,
            repo_id,
            scope_nonce: current_scope_nonce.get(),
            fallback_repo_id,
            issuer,
        })
        .await;
    match result {
        Ok(prepared) => channel.unicast(ServerMessage::RepoControl(
            RepoControlResponse::LocalRepoRemovalPrepared {
                request_id: prepared.request_id,
                preparation_id: prepared.preparation_id,
                repo_id: prepared.repo_id,
                preview: prepared.preview,
                confirmation_token: prepared.confirmation_token,
                fallback_binding: prepared.fallback_binding,
                expires_at_unix_ms: prepared.expires_at_unix_ms,
            },
        )),
        Err(error) => lifecycle::send_job_error(channel, request_id, &error),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_execute(
    state: &Arc<AppState>,
    channel: &DualChannel,
    session: &WsSession,
    request_id: Uuid,
    preparation_id: Uuid,
    confirmation_token: RemovalConfirmationToken,
    fallback_binding: Option<OpaqueFallbackBinding>,
    current_scope_nonce: ScopeNonce,
    switch_nonce: SwitchNonce,
) {
    let Some(issuer) = web_issuer(session) else {
        send_simple_error(
            channel,
            request_id,
            ServerErrorCode::RepoLifecycleConfirmationInvalid,
        );
        return;
    };
    if session.active_branch.is_some()
        || session.scope_nonce() != current_scope_nonce.get()
        || switch_nonce.get() <= current_scope_nonce.get()
    {
        send_simple_error(
            channel,
            request_id,
            ServerErrorCode::RepoLifecycleConfirmationStale,
        );
        return;
    }
    let Some(session_id) = session.repo_session_runtime_id() else {
        send_simple_error(
            channel,
            request_id,
            ServerErrorCode::RepoLifecycleConfirmationInvalid,
        );
        return;
    };
    if let Err(error) = state.repo_session_runtime().register_lifecycle_observer(
        session_id,
        request_id,
        current_scope_nonce.get(),
        switch_nonce.get(),
    ) {
        tracing::warn!(%error, "removal observer registration failed");
        send_simple_error(
            channel,
            request_id,
            ServerErrorCode::RepoLifecycleRepairRequired,
        );
        return;
    }
    let result = state
        .repo_lifecycle_jobs()
        .execute_removal(RepoRemovalExecuteIntent {
            request_id,
            preparation_id,
            confirmation_token,
            fallback_binding,
            scope_nonce: current_scope_nonce.get(),
            switch_nonce: switch_nonce.get(),
            issuer,
        })
        .await;
    match result {
        Ok(accepted) => {
            channel.unicast(ServerMessage::RepoControl(
                RepoControlResponse::LifecycleAccepted {
                    request_id: accepted.request_id,
                    job_id: accepted.job_id,
                    target_repo_id: accepted.target_repo_id,
                },
            ));
            if let Ok(status) = state.repo_lifecycle_jobs().status(request_id).await
                && status.phase
                    == crate::server::runtime::repo_lifecycle_job_runtime::RepoLifecycleJobPhase::Terminal
            {
                let _ = lifecycle::replay_terminal_status(state, session_id, request_id, &status);
                channel.unicast(ServerMessage::RepoControl(lifecycle::status_response(status)));
            }
        }
        Err(error) => {
            let _ = state
                .repo_session_runtime()
                .clear_lifecycle_observer(session_id, request_id);
            lifecycle::send_job_error(channel, request_id, &error);
        }
    }
}

fn web_issuer(session: &WsSession) -> Option<RepoRemovalIssuerBinding> {
    Some(RepoRemovalIssuerBinding::Web {
        principal_digest: session.auth_session_id()?.removal_binding_digest(),
        connection_epoch: session.repo_session_runtime_id()?,
    })
}
