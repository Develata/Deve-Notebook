//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!   - 14_commands#repo-removal-command-contract
//!
//! Explicit, one-shot resumption of an already committed removal plan.

use super::super::model::{
    RepoLifecycleJobAccepted, RepoLifecycleJobError, RepoLifecycleJobExecutor,
};
use super::super::removal::{
    RemovalRepairToken, RepoRemovalRepairApplyIntent, RepoRemovalRepairInspection,
    RepoRemovalRepairIssuerBinding, RepoRemovalRepairPrepared,
};
use super::super::store::ReceiptStore;
use super::{FinishedJob, MAX_ACTIVE_JOBS, admitted, removal_execution_for_request, spawn_job};
use crate::server::runtime::repo_lifecycle_job_runtime::removal::RemovalProgressCommand;
use deve_core::models::RepoId;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use uuid::Uuid;

const REPAIR_TOKEN_TTL_MS: i64 = 5 * 60 * 1_000;

pub(super) struct ApplyRepairContext<'a> {
    pub(super) store: &'a mut ReceiptStore,
    pub(super) executor: &'a Arc<dyn RepoLifecycleJobExecutor>,
    pub(super) jobs: &'a mut JoinSet<FinishedJob>,
    pub(super) active_repos: &'a mut HashSet<RepoId>,
    pub(super) progress_tx: mpsc::Sender<RemovalProgressCommand>,
}

pub(super) fn prepare(
    store: &mut ReceiptStore,
    executor: &dyn RepoLifecycleJobExecutor,
    active_repos: &HashSet<RepoId>,
    progress_tx: &mpsc::Sender<RemovalProgressCommand>,
    request_id: Uuid,
    issuer: RepoRemovalRepairIssuerBinding,
    now_ms: i64,
) -> Result<RepoRemovalRepairPrepared, RepoLifecycleJobError> {
    if request_id.is_nil() {
        return Err(RepoLifecycleJobError::InvalidRequest);
    }
    issuer.validate()?;
    store.clear_removal_repair_authorization(request_id)?;
    let removal = removal_execution_for_request(store, request_id, progress_tx.clone())?;
    if active_repos.contains(&removal.manifest.repo_id) {
        return Err(RepoLifecycleJobError::Busy);
    }
    let inspection = executor.inspect_removal_repair(&removal)?;
    let token = if inspection.apply_allowed {
        Some(random_token()?)
    } else {
        None
    };
    let expires_at_unix_ms = token
        .as_ref()
        .map(|_| now_ms.saturating_add(REPAIR_TOKEN_TTL_MS));
    if let (Some(token), Some(expires_at_unix_ms)) = (&token, expires_at_unix_ms) {
        store.publish_removal_repair_authorization(
            request_id,
            token_hash(token),
            inspection_digest(&inspection)?,
            issuer,
            expires_at_unix_ms,
        )?;
    }
    Ok(RepoRemovalRepairPrepared {
        inspection,
        token,
        expires_at_unix_ms,
    })
}

pub(super) fn apply(
    context: ApplyRepairContext<'_>,
    intent: RepoRemovalRepairApplyIntent,
    now_ms: i64,
) -> Result<RepoLifecycleJobAccepted, RepoLifecycleJobError> {
    if intent.request_id.is_nil() {
        return Err(RepoLifecycleJobError::InvalidRequest);
    }
    intent.issuer.validate()?;
    let ApplyRepairContext {
        store,
        executor,
        jobs,
        active_repos,
        progress_tx,
    } = context;
    let supplied_token_hash = token_hash(&intent.token);
    if let Some(receipt) = store.replay_consumed_removal_repair(
        intent.request_id,
        &supplied_token_hash,
        &intent.issuer,
    ) {
        return Ok(RepoLifecycleJobAccepted {
            request_id: receipt.request_id,
            job_id: receipt.job_id,
            target_repo_id: receipt.target_repo_id,
        });
    }
    let removal = removal_execution_for_request(store, intent.request_id, progress_tx.clone())?;
    if jobs.len() >= MAX_ACTIVE_JOBS || active_repos.contains(&removal.manifest.repo_id) {
        return Err(RepoLifecycleJobError::Busy);
    }
    let inspection = executor.inspect_removal_repair(&removal)?;
    if !inspection.apply_allowed {
        return Err(RepoLifecycleJobError::RemovalRepairBlocked);
    }
    let receipt = store.consume_removal_repair_authorization(
        intent.request_id,
        &supplied_token_hash,
        &inspection_digest(&inspection)?,
        &intent.issuer,
        now_ms,
    )?;
    active_repos.insert(receipt.target_repo_id);
    let removal = removal_execution_for_request(store, intent.request_id, progress_tx)?;
    spawn_job(
        jobs,
        executor.clone(),
        admitted(&receipt),
        Some(removal),
        true,
    );
    Ok(RepoLifecycleJobAccepted {
        request_id: receipt.request_id,
        job_id: receipt.job_id,
        target_repo_id: receipt.target_repo_id,
    })
}

fn random_token() -> Result<RemovalRepairToken, RepoLifecycleJobError> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|_| RepoLifecycleJobError::Coordination("secure random source unavailable"))?;
    let value = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    RemovalRepairToken::from_backend(value).ok_or(RepoLifecycleJobError::Coordination(
        "generated invalid repair token",
    ))
}

fn token_hash(token: &RemovalRepairToken) -> String {
    let mut hash = Sha256::new();
    hash.update(b"deve.local-repo-removal.repair-token.v1\0");
    hash.update(token.as_str().as_bytes());
    format!("{:x}", hash.finalize())
}

fn inspection_digest(
    inspection: &RepoRemovalRepairInspection,
) -> Result<String, RepoLifecycleJobError> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(inspection)?)
    ))
}
