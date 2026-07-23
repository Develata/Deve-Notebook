//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!   - 14_commands#repo-removal-command-contract

use super::output;
use super::token::CliRemovalToken;
use crate::server::local_repo_removal_cli_runtime::{OfflineRemovalClaim, OfflineRemovalRuntime};
use crate::server::{RemovalRepairToken, RepoLifecycleJobError};
use anyhow::{Result, anyhow};
use deve_core::ledger::RepoManager;
use deve_core::models::RepoId;
use deve_core::protocol::RepoLifecycleState;
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use uuid::Uuid;

pub(super) async fn run(
    repo: Arc<RepoManager>,
    repo_id: RepoId,
    encoded_token: Option<&str>,
) -> Result<()> {
    let runtime = OfflineRemovalRuntime::start(repo)?;
    let operation = match encoded_token {
        Some(encoded) => execute(&runtime, repo_id, encoded).await,
        None => prepare(&runtime, repo_id).await,
    };
    let shutdown = runtime.shutdown().await;
    match (operation, shutdown) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(()), Err(cleanup)) => Err(cleanup.context("offline removal runtime shutdown failed")),
        (Err(primary), Err(cleanup)) => Err(primary.context(format!(
            "offline removal runtime shutdown also failed: {cleanup}"
        ))),
    }
}

pub(super) async fn run_repair(
    repo: Arc<RepoManager>,
    claim: OfflineRemovalClaim,
    request_id: Uuid,
    encoded_token: Option<&str>,
) -> Result<()> {
    let runtime = OfflineRemovalRuntime::start_repair(repo, claim, request_id)?;
    let operation = match encoded_token {
        Some(encoded) => apply_repair(&runtime, request_id, encoded).await,
        None => prepare_repair(&runtime, request_id).await,
    };
    let shutdown = runtime.shutdown().await;
    match (operation, shutdown) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(()), Err(cleanup)) => Err(cleanup.context("offline repair runtime shutdown failed")),
        (Err(primary), Err(cleanup)) => Err(primary.context(format!(
            "offline repair runtime shutdown also failed: {cleanup}"
        ))),
    }
}

async fn prepare(runtime: &OfflineRemovalRuntime, repo_id: RepoId) -> Result<()> {
    let prepared = runtime.prepare(repo_id).await?;
    if prepared.repo_id != repo_id {
        return Err(anyhow!("REPO_LIFECYCLE_REPAIR_REQUIRED"));
    }
    let token = prepared
        .confirmation_token
        .map(|secret| CliRemovalToken::issue(repo_id, prepared.preparation_id, secret));
    output::prepared(repo_id, prepared.preparation_id, &prepared.preview, token)
}

async fn execute(
    runtime: &OfflineRemovalRuntime,
    repo_id: RepoId,
    encoded_token: &str,
) -> Result<()> {
    let token = CliRemovalToken::parse(encoded_token, repo_id)?;
    let accepted = runtime
        .execute(
            repo_id,
            token.preparation_id,
            token.execute_request_id,
            token.confirmation,
        )
        .await?;
    if accepted.repo_id != repo_id {
        return Err(anyhow!("REPO_LIFECYCLE_REPAIR_REQUIRED"));
    }
    output::accepted(repo_id, accepted.request_id, accepted.job_id);
    wait_terminal(runtime, accepted.request_id).await
}

async fn prepare_repair(runtime: &OfflineRemovalRuntime, request_id: Uuid) -> Result<()> {
    let prepared = match runtime.prepare_repair(request_id).await {
        Ok(prepared) => prepared,
        Err(error)
            if error.chain().any(|cause| {
                matches!(
                    cause.downcast_ref::<RepoLifecycleJobError>(),
                    Some(RepoLifecycleJobError::RemovalRepairNotRequired)
                )
            }) =>
        {
            return wait_terminal(runtime, request_id).await;
        }
        Err(error) => return Err(error),
    };
    output::repair_prepared(
        &prepared.inspection,
        prepared.token.as_ref().map(RemovalRepairToken::as_str),
        prepared.expires_at_unix_ms,
    )
}

async fn apply_repair(
    runtime: &OfflineRemovalRuntime,
    request_id: Uuid,
    encoded_token: &str,
) -> Result<()> {
    let token = RemovalRepairToken::from_backend(encoded_token.to_owned())
        .ok_or_else(|| anyhow!("REPO_LIFECYCLE_CONFIRMATION_INVALID"))?;
    let accepted = runtime.apply_repair(request_id, token).await?;
    output::repair_accepted(accepted.repo_id, accepted.request_id, accepted.job_id);
    wait_terminal(runtime, accepted.request_id).await
}

async fn wait_terminal(runtime: &OfflineRemovalRuntime, request_id: Uuid) -> Result<()> {
    loop {
        let status = runtime
            .status_if_known(request_id)
            .await?
            .ok_or_else(|| anyhow!("REPO_LIFECYCLE_NOT_FOUND"))?;
        if status.state == RepoLifecycleState::Terminal {
            return output::terminal(
                status.repo_id,
                status.request_id,
                status.state,
                status.outcome,
                status.publication_pending,
            );
        }
        sleep(Duration::from_millis(100)).await;
    }
}
