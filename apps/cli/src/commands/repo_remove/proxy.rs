//! plan_ref:
//!   - 08_auth#local-cli-proxy-authority
//!   - 14_commands#repo-removal-command-contract

use super::output;
use super::token::CliRemovalToken;
use crate::commands::live_proxy;
use crate::commands::remote_import::LocalCliAuthArgs;
use crate::local_cli_proxy_contract::{LocalCliRepoRemovalRequest, LocalCliRepoRemovalResponse};
use anyhow::{Context, Result, anyhow, bail};
use deve_core::models::RepoId;
use deve_core::protocol::{RepoLifecycleState, ScopeNonce, ServerErrorCode, SwitchNonce};
use std::path::Path;
use tokio::time::{Duration, sleep};
use uuid::Uuid;

const SCOPE_NONCE: u64 = 1;
const SWITCH_NONCE: u64 = 2;

pub(super) async fn run(
    ledger_dir: &Path,
    repo_id: RepoId,
    encoded_token: Option<&str>,
    auth: LocalCliAuthArgs,
) -> Result<()> {
    let session =
        live_proxy::authenticated_session(ledger_dir, auth.auth_user, auth.auth_password_stdin)
            .await?;
    match encoded_token {
        Some(encoded) => execute(&session, repo_id, encoded).await,
        None => prepare(&session, repo_id).await,
    }
}

pub(super) async fn run_repair(
    ledger_dir: &Path,
    request_id: Uuid,
    encoded_token: Option<&str>,
    auth: LocalCliAuthArgs,
) -> Result<()> {
    let session =
        live_proxy::authenticated_session(ledger_dir, auth.auth_user, auth.auth_password_stdin)
            .await?;
    match encoded_token {
        Some(token) => apply_repair(&session, request_id, token).await,
        None => prepare_repair(&session, request_id).await,
    }
}

async fn prepare(session: &live_proxy::LocalCliProxySession, repo_id: RepoId) -> Result<()> {
    let response = send(
        session,
        &LocalCliRepoRemovalRequest::Prepare {
            request_id: Uuid::new_v4(),
            repo_id,
            current_scope_nonce: ScopeNonce::new(SCOPE_NONCE),
        },
    )
    .await?;
    match response {
        LocalCliRepoRemovalResponse::Prepared {
            preparation_id,
            repo_id: actual_repo_id,
            preview,
            confirmation_token,
            fallback_binding,
            ..
        } if actual_repo_id == repo_id && fallback_binding.is_none() => {
            let token = confirmation_token
                .map(|secret| CliRemovalToken::issue(repo_id, preparation_id, secret));
            output::prepared(repo_id, preparation_id, &preview, token)
        }
        LocalCliRepoRemovalResponse::Error { error, .. } => server_error(error),
        _ => bail!("REPO_LIFECYCLE_REPAIR_REQUIRED"),
    }
}

async fn execute(
    session: &live_proxy::LocalCliProxySession,
    repo_id: RepoId,
    encoded_token: &str,
) -> Result<()> {
    let token = CliRemovalToken::parse(encoded_token, repo_id)?;
    let response = send(
        session,
        &LocalCliRepoRemovalRequest::Execute {
            request_id: token.execute_request_id,
            repo_id,
            preparation_id: token.preparation_id,
            confirmation_token: token.confirmation,
            fallback_binding: None,
            current_scope_nonce: ScopeNonce::new(SCOPE_NONCE),
            switch_nonce: SwitchNonce::new(SWITCH_NONCE),
        },
    )
    .await?;
    match response {
        LocalCliRepoRemovalResponse::Accepted {
            request_id,
            job_id,
            repo_id: actual_repo_id,
        } if request_id == token.execute_request_id && actual_repo_id == repo_id => {
            output::accepted(repo_id, request_id, job_id);
            poll_status(session, repo_id, request_id).await
        }
        LocalCliRepoRemovalResponse::Error { error, .. } => server_error(error),
        _ => bail!("REPO_LIFECYCLE_REPAIR_REQUIRED"),
    }
}

async fn prepare_repair(
    session: &live_proxy::LocalCliProxySession,
    request_id: Uuid,
) -> Result<()> {
    loop {
        let response = send(
            session,
            &LocalCliRepoRemovalRequest::RepairPrepare { request_id },
        )
        .await?;
        match response {
            LocalCliRepoRemovalResponse::RepairPrepared {
                request_id: actual_request_id,
                inspection,
                token,
                expires_at_unix_ms,
            } if actual_request_id == request_id && inspection.request_id == request_id => {
                return output::repair_prepared(&inspection, token.as_deref(), expires_at_unix_ms);
            }
            LocalCliRepoRemovalResponse::Status {
                execute_request_id,
                repo_id,
                state,
                outcome,
                publication_pending,
                ..
            } if execute_request_id == request_id && state == RepoLifecycleState::Terminal => {
                return output::terminal(repo_id, request_id, state, outcome, publication_pending);
            }
            LocalCliRepoRemovalResponse::Error { error, .. }
                if error.code == ServerErrorCode::RepoLifecycleBusy =>
            {
                sleep(Duration::from_millis(100)).await;
            }
            LocalCliRepoRemovalResponse::Error { error, .. } => return server_error(error),
            _ => bail!("REPO_LIFECYCLE_REPAIR_REQUIRED"),
        }
    }
}

async fn apply_repair(
    session: &live_proxy::LocalCliProxySession,
    request_id: Uuid,
    token: &str,
) -> Result<()> {
    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("REPO_LIFECYCLE_CONFIRMATION_INVALID");
    }
    let response = send(
        session,
        &LocalCliRepoRemovalRequest::RepairApply {
            request_id,
            token: token.to_owned(),
        },
    )
    .await?;
    match response {
        LocalCliRepoRemovalResponse::Accepted {
            request_id: actual_request_id,
            job_id,
            repo_id,
        } if actual_request_id == request_id && !repo_id.is_nil() => {
            output::repair_accepted(repo_id, request_id, job_id);
            poll_status(session, repo_id, request_id).await
        }
        LocalCliRepoRemovalResponse::Error { error, .. } => server_error(error),
        _ => bail!("REPO_LIFECYCLE_REPAIR_REQUIRED"),
    }
}

async fn poll_status(
    session: &live_proxy::LocalCliProxySession,
    repo_id: RepoId,
    execute_request_id: Uuid,
) -> Result<()> {
    let status_request_id = Uuid::new_v4();
    loop {
        let response = send(
            session,
            &LocalCliRepoRemovalRequest::Status {
                request_id: status_request_id,
                execute_request_id,
                repo_id,
            },
        )
        .await?;
        match response {
            LocalCliRepoRemovalResponse::Status {
                request_id,
                execute_request_id: actual_execute_request_id,
                repo_id: actual_repo_id,
                operation,
                state,
                outcome,
                publication_pending,
                ..
            } if request_id == status_request_id
                && actual_execute_request_id == execute_request_id
                && actual_repo_id == repo_id =>
            {
                if operation != deve_core::protocol::RepoLifecycleOperation::Remove {
                    bail!("REPO_LIFECYCLE_REPAIR_REQUIRED");
                }
                if state == deve_core::protocol::RepoLifecycleState::Terminal {
                    return output::terminal(
                        repo_id,
                        execute_request_id,
                        state,
                        outcome,
                        publication_pending,
                    );
                }
            }
            LocalCliRepoRemovalResponse::Error { error, .. } => return server_error(error),
            _ => bail!("REPO_LIFECYCLE_REPAIR_REQUIRED"),
        }
        sleep(Duration::from_millis(100)).await;
    }
}

async fn send(
    session: &live_proxy::LocalCliProxySession,
    request: &LocalCliRepoRemovalRequest,
) -> Result<LocalCliRepoRemovalResponse> {
    let response = session
        .post("/api/local-cli/repo-removal")
        .json(request)
        .send()
        .await
        .context("Local CLI repo removal proxy request failed")?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .context("Local CLI repo removal proxy response body failed")?;
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        bail!(live_proxy::decode_auth_rejection(&bytes)?);
    }
    serde_json::from_slice(&bytes).map_err(|_| anyhow!("REPO_LIFECYCLE_REPAIR_REQUIRED"))
}

fn server_error(error: deve_core::protocol::ServerError) -> Result<()> {
    let code = serde_json::to_string(&error.code)?;
    Err(output::symbolic_error(code.trim_matches('"').to_string()))
}
