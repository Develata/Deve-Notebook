//! plan_ref:
//!   - 08_auth#local-cli-proxy-authority
//!   - 14_commands#remote-import-command-contract
//!
//! Dedicated Local CLI proxy endpoint. Authentication is performed by the
//! crate-private gateway before repository or provider access.

mod identity;
mod intent;
mod response;

use super::{RemoteImportPhase, host_error_code, run_blocking};
use crate::local_cli_proxy_contract::{LocalCliRemoteImportRequest, LocalCliRemoteImportResponse};
use crate::server::AppState;
use crate::server::auth::local_cli_proxy::LocalCliProxyGateway;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use deve_core::protocol::auth::AuthErrorResponse;
use deve_core::protocol::{RemoteImportRequestContext, ServerErrorCode};
use deve_core::security::AuthConfig;
use intent::execute_intent;
use response::{auth_internal_error, proxy_error};
use std::net::SocketAddr;
use std::sync::Arc;

pub(crate) async fn local_cli_proxy(
    State(state): State<Arc<AppState>>,
    Extension(config): Extension<Arc<AuthConfig>>,
    Extension(gateway): Extension<Arc<LocalCliProxyGateway>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let (authority, request) = match gateway.admit(peer, &headers, &config, &body) {
        Ok(admitted) => admitted,
        Err(rejection) => {
            return (
                rejection.status,
                Json(AuthErrorResponse::new(rejection.code)),
            )
                .into_response();
        }
    };
    if authority.request_id() != request.request_id()
        || authority.operation() != request.operation_name()
    {
        return auth_internal_error();
    }
    let repo_name = match state
        .repo
        .resolve_local_repo_name_for_execution(Some(authority.repo_id()), None)
    {
        Ok(repo_name) => repo_name,
        Err(error) => {
            tracing::warn!(%error, "Local CLI proxy repo identity revalidation failed");
            return proxy_error(
                authority.request_id(),
                ServerErrorCode::RemoteImportInvalidState,
            );
        }
    };
    execute(state, repo_name, request).await
}

async fn execute(
    state: Arc<AppState>,
    repo_name: String,
    request: LocalCliRemoteImportRequest,
) -> Response {
    match request {
        LocalCliRemoteImportRequest::Intent { request } => {
            execute_intent(state, repo_name, request).await
        }
        LocalCliRemoteImportRequest::Repair { context, apply } => {
            execute_repair(state, context, apply).await
        }
    }
}

async fn execute_repair(
    state: Arc<AppState>,
    context: RemoteImportRequestContext,
    apply: bool,
) -> Response {
    if context.branch.is_some() || context.scope_nonce.get() == 0 {
        return proxy_error(
            context.request_id,
            ServerErrorCode::RemoteImportInvalidState,
        );
    }
    let coordinator = state.remote_import_coordinator();
    let result = run_blocking(move || {
        let plan = coordinator.inspect_repair(context.repo_id)?;
        if apply {
            coordinator.apply_repair(context.repo_id, plan.token())
        } else {
            Ok(plan)
        }
    })
    .await;
    match result {
        Ok(plan) => (
            StatusCode::OK,
            Json(LocalCliRemoteImportResponse::Repair {
                request_id: context.request_id,
                repo_id: context.repo_id,
                branch: context.branch,
                scope_nonce: context.scope_nonce,
                finding_count: plan.finding_count,
                repairable_count: plan.repairable_count,
            }),
        )
            .into_response(),
        Err(error) => {
            tracing::warn!(%error, "Local CLI Remote Import repair failed");
            proxy_error(
                context.request_id,
                host_error_code(&error, RemoteImportPhase::Prepare),
            )
        }
    }
}
