//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!   - 08_auth#local-cli-proxy-authority
//!   - 14_commands#repo-removal-command-contract
//!
//! Loopback CLI transport for the same lifecycle owner used by WebSocket
//! clients. Authentication and replay admission complete before any lifecycle
//! lookup or mutation.

use super::lifecycle;
use crate::local_cli_proxy_contract::{LocalCliRepoRemovalRequest, LocalCliRepoRemovalResponse};
use crate::server::AppState;
use crate::server::auth::local_cli_proxy::LocalCliProxyGateway;
use crate::server::runtime::repo_lifecycle_job_runtime::{
    RemovalRepairToken, RepoLifecycleJobOperation as HostOperation,
    RepoLifecycleJobOutcome as HostOutcome, RepoLifecycleJobPhase as HostPhase,
    RepoRemovalExecuteIntent, RepoRemovalIssuerBinding, RepoRemovalPrepareIntent,
    RepoRemovalRepairApplyIntent, RepoRemovalRepairIssuerBinding,
};
use axum::body::Bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use deve_core::protocol::auth::AuthErrorResponse;
use deve_core::protocol::{
    RepoLifecycleOperation, RepoLifecycleOutcome, RepoLifecycleState, ServerError, ServerErrorCode,
};
use deve_core::security::AuthConfig;
use std::net::SocketAddr;
use std::sync::Arc;
use uuid::Uuid;

pub(crate) async fn local_cli_proxy(
    State(state): State<Arc<AppState>>,
    Extension(config): Extension<Arc<AuthConfig>>,
    Extension(gateway): Extension<Arc<LocalCliProxyGateway>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let (authority, request) = match gateway.admit_repo_removal(peer, &headers, &config, &body) {
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
        || authority.repo_id() != request.repo_id()
        || authority.operation() != request.operation_name()
    {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            request.request_id(),
            ServerErrorCode::RepoLifecycleRepairRequired,
        );
    }
    let principal_digest = authority.principal_digest().to_string();
    let issuer = RepoRemovalIssuerBinding::LocalCliProxy {
        principal_digest: principal_digest.clone(),
    };
    let repair_issuer = RepoRemovalRepairIssuerBinding::LocalCliProxy {
        principal_digest,
        runtime_incarnation: state.repo_lifecycle_jobs().runtime_incarnation(),
    };
    execute(state, request, issuer, repair_issuer).await
}

async fn execute(
    state: Arc<AppState>,
    request: LocalCliRepoRemovalRequest,
    issuer: RepoRemovalIssuerBinding,
    repair_issuer: RepoRemovalRepairIssuerBinding,
) -> Response {
    match request {
        LocalCliRepoRemovalRequest::Prepare {
            request_id,
            repo_id,
            current_scope_nonce,
        } => match state
            .repo_lifecycle_jobs()
            .prepare_removal(RepoRemovalPrepareIntent {
                request_id,
                repo_id,
                scope_nonce: current_scope_nonce.get(),
                fallback_repo_id: None,
                issuer,
            })
            .await
        {
            Ok(prepared) => (
                StatusCode::OK,
                Json(LocalCliRepoRemovalResponse::Prepared {
                    request_id: prepared.request_id,
                    preparation_id: prepared.preparation_id,
                    repo_id: prepared.repo_id,
                    preview: prepared.preview,
                    confirmation_token: prepared.confirmation_token,
                    fallback_binding: prepared.fallback_binding,
                }),
            )
                .into_response(),
            Err(error) => job_error_response(request_id, &error),
        },
        LocalCliRepoRemovalRequest::Execute {
            request_id,
            repo_id,
            preparation_id,
            confirmation_token,
            fallback_binding,
            current_scope_nonce,
            switch_nonce,
        } => match state
            .repo_lifecycle_jobs()
            .execute_removal(RepoRemovalExecuteIntent {
                request_id,
                expected_repo_id: Some(repo_id),
                preparation_id,
                confirmation_token,
                fallback_binding,
                scope_nonce: current_scope_nonce.get(),
                switch_nonce: switch_nonce.get(),
                issuer,
            })
            .await
        {
            Ok(accepted) if accepted.target_repo_id == repo_id => (
                StatusCode::ACCEPTED,
                Json(LocalCliRepoRemovalResponse::Accepted {
                    request_id: accepted.request_id,
                    job_id: accepted.job_id,
                    repo_id: accepted.target_repo_id,
                }),
            )
                .into_response(),
            Ok(accepted) => {
                tracing::error!(
                    expected_repo_id = %repo_id,
                    actual_repo_id = %accepted.target_repo_id,
                    %request_id,
                    "Local CLI removal identity diverged after lifecycle admission"
                );
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    request_id,
                    ServerErrorCode::RepoLifecycleRepairRequired,
                )
            }
            Err(error) => job_error_response(request_id, &error),
        },
        LocalCliRepoRemovalRequest::Status {
            request_id,
            execute_request_id,
            repo_id,
        } => match state.repo_lifecycle_jobs().status(execute_request_id).await {
            Ok(status)
                if status_matches_removal(repo_id, status.target_repo_id, status.operation) =>
            {
                (
                    StatusCode::OK,
                    Json(LocalCliRepoRemovalResponse::Status {
                        request_id,
                        execute_request_id,
                        job_id: status.job_id,
                        repo_id: status.target_repo_id,
                        operation: match status.operation {
                            HostOperation::Create => RepoLifecycleOperation::Create,
                            HostOperation::Remove => RepoLifecycleOperation::Remove,
                        },
                        state: match status.phase {
                            HostPhase::Accepted => RepoLifecycleState::Accepted,
                            HostPhase::Running => RepoLifecycleState::Running,
                            HostPhase::Recovering => RepoLifecycleState::Recovering,
                            HostPhase::Terminal => RepoLifecycleState::Terminal,
                        },
                        outcome: status.outcome.map(|outcome| match outcome {
                            HostOutcome::Succeeded => RepoLifecycleOutcome::Succeeded,
                            HostOutcome::NotCommitted => RepoLifecycleOutcome::NotCommitted,
                            HostOutcome::CommittedPartial => RepoLifecycleOutcome::CommittedPartial,
                            HostOutcome::RepairRequired => RepoLifecycleOutcome::RepairRequired,
                        }),
                        publication_pending: status.publication_pending,
                    }),
                )
                    .into_response()
            }
            Ok(_) => error_response(
                StatusCode::CONFLICT,
                request_id,
                ServerErrorCode::RepoLifecycleInvalidRequest,
            ),
            Err(error) => job_error_response(request_id, &error),
        },
        LocalCliRepoRemovalRequest::RepairPrepare { request_id } => {
            match state
                .repo_lifecycle_jobs()
                .prepare_removal_repair(request_id, repair_issuer)
                .await
            {
                Ok(prepared) => (
                    StatusCode::OK,
                    Json(LocalCliRepoRemovalResponse::RepairPrepared {
                        request_id,
                        inspection: prepared.inspection,
                        token: prepared.token.map(|token| token.as_str().to_string()),
                        expires_at_unix_ms: prepared.expires_at_unix_ms,
                    }),
                )
                    .into_response(),
                Err(
                    crate::server::runtime::repo_lifecycle_job_runtime::RepoLifecycleJobError::RemovalRepairNotRequired,
                ) => match state.repo_lifecycle_jobs().status(request_id).await {
                    Ok(status) => (
                        StatusCode::OK,
                        Json(LocalCliRepoRemovalResponse::Status {
                            request_id,
                            execute_request_id: request_id,
                            job_id: status.job_id,
                            repo_id: status.target_repo_id,
                            operation: RepoLifecycleOperation::Remove,
                            state: match status.phase {
                                HostPhase::Accepted => RepoLifecycleState::Accepted,
                                HostPhase::Running => RepoLifecycleState::Running,
                                HostPhase::Recovering => RepoLifecycleState::Recovering,
                                HostPhase::Terminal => RepoLifecycleState::Terminal,
                            },
                            outcome: status.outcome.map(|outcome| match outcome {
                                HostOutcome::Succeeded => RepoLifecycleOutcome::Succeeded,
                                HostOutcome::NotCommitted => RepoLifecycleOutcome::NotCommitted,
                                HostOutcome::CommittedPartial => {
                                    RepoLifecycleOutcome::CommittedPartial
                                }
                                HostOutcome::RepairRequired => {
                                    RepoLifecycleOutcome::RepairRequired
                                }
                            }),
                            publication_pending: status.publication_pending,
                        }),
                    )
                        .into_response(),
                    Err(error) => job_error_response(request_id, &error),
                },
                Err(error) => job_error_response(request_id, &error),
            }
        }
        LocalCliRepoRemovalRequest::RepairApply { request_id, token } => {
            let Some(token) = RemovalRepairToken::from_backend(token) else {
                return error_response(
                    StatusCode::CONFLICT,
                    request_id,
                    ServerErrorCode::RepoLifecycleConfirmationInvalid,
                );
            };
            match state
                .repo_lifecycle_jobs()
                .apply_removal_repair(RepoRemovalRepairApplyIntent {
                    request_id,
                    token,
                    issuer: repair_issuer,
                })
                .await
            {
                Ok(accepted) => (
                    StatusCode::ACCEPTED,
                    Json(LocalCliRepoRemovalResponse::Accepted {
                        request_id: accepted.request_id,
                        job_id: accepted.job_id,
                        repo_id: accepted.target_repo_id,
                    }),
                )
                    .into_response(),
                Err(error) => job_error_response(request_id, &error),
            }
        }
    }
}

fn status_matches_removal(
    expected_repo_id: deve_core::models::RepoId,
    actual_repo_id: deve_core::models::RepoId,
    operation: HostOperation,
) -> bool {
    expected_repo_id == actual_repo_id && operation == HostOperation::Remove
}

fn job_error_response(
    request_id: Uuid,
    error: &crate::server::runtime::repo_lifecycle_job_runtime::RepoLifecycleJobError,
) -> Response {
    tracing::warn!(%request_id, %error, "Local CLI repo removal request failed");
    let status = match error {
        crate::server::runtime::repo_lifecycle_job_runtime::RepoLifecycleJobError::NotFound => {
            StatusCode::NOT_FOUND
        }
        crate::server::runtime::repo_lifecycle_job_runtime::RepoLifecycleJobError::Busy
        | crate::server::runtime::repo_lifecycle_job_runtime::RepoLifecycleJobError::OwnerActive => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        crate::server::runtime::repo_lifecycle_job_runtime::RepoLifecycleJobError::Store(_)
        | crate::server::runtime::repo_lifecycle_job_runtime::RepoLifecycleJobError::Coordination(_)
        | crate::server::runtime::repo_lifecycle_job_runtime::RepoLifecycleJobError::Shutdown(_)
        | crate::server::runtime::repo_lifecycle_job_runtime::RepoLifecycleJobError::AdmissionClosed => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
        _ => StatusCode::CONFLICT,
    };
    error_response(status, request_id, lifecycle::job_error_code(error))
}

fn error_response(
    status: StatusCode,
    request_id: Uuid,
    code: deve_core::protocol::ServerErrorCode,
) -> Response {
    (
        status,
        Json(LocalCliRepoRemovalResponse::Error {
            request_id,
            error: ServerError::new(code),
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removal_status_rejects_create_or_foreign_repo_receipts() {
        let repo_id = uuid::Uuid::new_v4();
        assert!(status_matches_removal(
            repo_id,
            repo_id,
            HostOperation::Remove
        ));
        assert!(!status_matches_removal(
            repo_id,
            repo_id,
            HostOperation::Create
        ));
        assert!(!status_matches_removal(
            repo_id,
            uuid::Uuid::new_v4(),
            HostOperation::Remove
        ));
    }
}
