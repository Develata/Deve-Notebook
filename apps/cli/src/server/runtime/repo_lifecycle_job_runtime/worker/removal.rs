//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!   - 07_network#repo-control-wire-contract
//!
//! Serialized Prepare/Execute admission inside the lifecycle worker.

use super::{FinishedJob, MAX_ACTIVE_JOBS, admitted, spawn_job};
use crate::server::runtime::repo_lifecycle_job_runtime::model::{
    RepoLifecycleJobAccepted, RepoLifecycleJobError, RepoLifecycleJobExecutor,
    RepoLifecycleJobIntent,
};
use crate::server::runtime::repo_lifecycle_job_runtime::removal::{
    RepoRemovalExecuteIntent, RepoRemovalPrepareIntent, RepoRemovalPrepared,
};
use crate::server::runtime::repo_lifecycle_job_runtime::store::{
    LifecycleReceipt, ReceiptStore, RemovalPreparationRecord, RemovalPreparationState,
};
use deve_core::models::RepoId;
use deve_core::protocol::{OpaqueFallbackBinding, RemovalConfirmationToken};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use uuid::Uuid;

const CONFIRMATION_TTL_MS: i64 = 5 * 60 * 1_000;

pub(super) struct ExecuteRemovalContext<'a> {
    pub(super) store: &'a mut ReceiptStore,
    pub(super) executor: &'a Arc<dyn RepoLifecycleJobExecutor>,
    pub(super) jobs: &'a mut JoinSet<FinishedJob>,
    pub(super) active_repos: &'a mut HashSet<RepoId>,
    pub(super) runtime_incarnation: Uuid,
    pub(super) progress_tx: mpsc::Sender<super::RemovalProgressCommand>,
}

pub(super) async fn prepare_removal(
    store: &mut ReceiptStore,
    executor: &dyn RepoLifecycleJobExecutor,
    runtime_incarnation: Uuid,
    intent: RepoRemovalPrepareIntent,
) -> Result<RepoRemovalPrepared, RepoLifecycleJobError> {
    if intent.request_id.is_nil() || intent.scope_nonce == u64::MAX {
        return Err(RepoLifecycleJobError::InvalidRequest);
    }
    intent.issuer.validate()?;
    let existing_id = if let Some(existing) = store.removal_by_prepare_request(intent.request_id) {
        if existing.repo_id != intent.repo_id
            || existing.scope_nonce != intent.scope_nonce
            || existing.fallback_repo_id != intent.fallback_repo_id
            || existing.issuer != intent.issuer
            || matches!(
                existing.state,
                RemovalPreparationState::ExecuteAdmitted { .. }
            )
        {
            return Err(RepoLifecycleJobError::RequestConflict);
        }
        Some(existing.preparation_id)
    } else {
        if store.request_id_is_bound(intent.request_id) {
            return Err(RepoLifecycleJobError::RequestConflict);
        }
        None
    };
    let prepared = executor.prepare_removal(intent.clone()).await?;
    if !intent.issuer.binds_manifest(prepared.manifest.as_ref()) {
        return Err(RepoLifecycleJobError::InvalidRequest);
    }
    let manifest_digest = prepared
        .manifest
        .as_ref()
        .map(crate::server::runtime::repo_lifecycle_job_runtime::store::removal::manifest_digest)
        .transpose()?;
    let token = if prepared.preview.blockers.is_empty() && prepared.manifest.is_some() {
        Some(random_token()?)
    } else {
        None
    };
    let fallback_binding = if token.is_some()
        && prepared
            .manifest
            .as_ref()
            .is_some_and(|manifest| manifest.fallback.is_some())
    {
        Some(random_fallback_binding()?)
    } else {
        None
    };
    let now = chrono::Utc::now().timestamp_millis();
    let expires_at_unix_ms = token
        .as_ref()
        .map(|_| now.saturating_add(CONFIRMATION_TTL_MS));
    let preparation_id = existing_id.unwrap_or_else(Uuid::new_v4);
    let record = RemovalPreparationRecord::prepared(
        intent.request_id,
        preparation_id,
        intent.repo_id,
        intent.scope_nonce,
        intent.fallback_repo_id,
        intent.issuer,
        runtime_incarnation,
        manifest_digest,
        prepared.manifest,
        prepared.preview.clone(),
        token.as_ref().map(token_hash),
        fallback_binding.as_ref().map(fallback_hash),
        expires_at_unix_ms.unwrap_or(now),
    );
    store.publish_preparation(record)?;
    store.prune_removals_only()?;
    Ok(RepoRemovalPrepared {
        request_id: intent.request_id,
        preparation_id,
        repo_id: intent.repo_id,
        preview: prepared.preview,
        confirmation_token: token,
        fallback_binding,
        expires_at_unix_ms,
    })
}

pub(super) async fn execute_removal(
    context: ExecuteRemovalContext<'_>,
    now_ms: i64,
    intent: RepoRemovalExecuteIntent,
) -> Result<RepoLifecycleJobAccepted, RepoLifecycleJobError> {
    let ExecuteRemovalContext {
        store,
        executor,
        jobs,
        active_repos,
        runtime_incarnation,
        progress_tx,
    } = context;
    intent.issuer.validate()?;
    if let Some(record) = store.removal_by_execute_request(intent.request_id) {
        let supplied_token_hash = token_hash(&intent.confirmation_token);
        let supplied_fallback_hash = intent.fallback_binding.as_ref().map(fallback_hash);
        let exact = record.preparation_id == intent.preparation_id
            && intent
                .expected_repo_id
                .is_none_or(|repo_id| repo_id == record.repo_id)
            && record.scope_nonce == intent.scope_nonce
            && record.issuer == intent.issuer
            && matches!(
                &record.state,
                RemovalPreparationState::ExecuteAdmitted {
                    consumed_token_hash,
                    consumed_fallback_hash,
                    switch_nonce,
                    ..
                } if constant_time_eq(consumed_token_hash.as_bytes(), supplied_token_hash.as_bytes())
                    && optional_hash_eq(
                        consumed_fallback_hash.as_deref(),
                        supplied_fallback_hash.as_deref(),
                    )
                    && *switch_nonce == intent.switch_nonce
            );
        if !exact {
            return Err(RepoLifecycleJobError::RequestConflict);
        }
        return record
            .receipt_for_request(intent.request_id)
            .map(accepted)
            .ok_or(RepoLifecycleJobError::NotFound);
    }
    if intent.request_id.is_nil() || store.request_id_is_bound(intent.request_id) {
        return Err(RepoLifecycleJobError::RequestConflict);
    }
    let record = store
        .removal(intent.preparation_id)
        .cloned()
        .ok_or(RepoLifecycleJobError::NotFound)?;
    if intent
        .expected_repo_id
        .is_some_and(|repo_id| repo_id != record.repo_id)
    {
        return Err(RepoLifecycleJobError::ConfirmationInvalid);
    }
    if (record.issuer.is_runtime_bound() && record.runtime_incarnation != runtime_incarnation)
        || record.issuer != intent.issuer
        || record.scope_nonce != intent.scope_nonce
        || intent.switch_nonce <= intent.scope_nonce
    {
        return Err(RepoLifecycleJobError::ConfirmationInvalid);
    }
    let (expected_token_hash, expected_fallback_hash) = match &record.state {
        RemovalPreparationState::Prepared {
            token_hash: Some(token_hash),
            fallback_binding_hash,
        } => (token_hash, fallback_binding_hash),
        _ => return Err(RepoLifecycleJobError::ConfirmationInvalid),
    };
    if now_ms >= record.expires_at_unix_ms {
        return Err(RepoLifecycleJobError::ConfirmationExpired);
    }
    let supplied_token_hash = token_hash(&intent.confirmation_token);
    if !constant_time_eq(
        expected_token_hash.as_bytes(),
        supplied_token_hash.as_bytes(),
    ) {
        return Err(RepoLifecycleJobError::ConfirmationInvalid);
    }
    let supplied_fallback_hash = intent.fallback_binding.as_ref().map(fallback_hash);
    if !optional_hash_eq(
        expected_fallback_hash.as_deref(),
        supplied_fallback_hash.as_deref(),
    ) {
        return Err(RepoLifecycleJobError::ConfirmationInvalid);
    }
    let manifest = record
        .manifest
        .clone()
        .ok_or(RepoLifecycleJobError::RemovalBlocked)?;
    executor.revalidate_removal(manifest).await?;
    if jobs.len() >= MAX_ACTIVE_JOBS || active_repos.contains(&record.repo_id) {
        return Err(RepoLifecycleJobError::Busy);
    }
    let job_id = Uuid::new_v4();
    let receipt = LifecycleReceipt::admitted(
        intent.request_id,
        job_id,
        record.repo_id,
        RepoLifecycleJobIntent::remove(record.repo_id),
    )?;
    let receipt = store.admit_prepared_removal(
        intent.preparation_id,
        intent.request_id,
        supplied_token_hash,
        supplied_fallback_hash,
        intent.switch_nonce,
        receipt,
    )?;
    active_repos.insert(record.repo_id);
    let removal = super::removal_execution_for_request(store, receipt.request_id, progress_tx)?;
    spawn_job(
        jobs,
        executor.clone(),
        admitted(&receipt),
        Some(removal),
        false,
    );
    Ok(accepted(&receipt))
}

fn accepted(receipt: &LifecycleReceipt) -> RepoLifecycleJobAccepted {
    RepoLifecycleJobAccepted {
        request_id: receipt.request_id,
        job_id: receipt.job_id,
        target_repo_id: receipt.target_repo_id,
    }
}

fn random_token() -> Result<RemovalConfirmationToken, RepoLifecycleJobError> {
    RemovalConfirmationToken::from_backend(random_hex_256()?).ok_or(
        RepoLifecycleJobError::Coordination("generated invalid removal token"),
    )
}

fn random_fallback_binding() -> Result<OpaqueFallbackBinding, RepoLifecycleJobError> {
    OpaqueFallbackBinding::from_backend(random_hex_256()?).ok_or(
        RepoLifecycleJobError::Coordination("generated invalid fallback binding"),
    )
}

fn random_hex_256() -> Result<String, RepoLifecycleJobError> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|_| RepoLifecycleJobError::Coordination("secure random source unavailable"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn token_hash(token: &RemovalConfirmationToken) -> String {
    domain_hash(b"deve.local-repo-removal.token.v1\0", token.as_str())
}

fn fallback_hash(binding: &OpaqueFallbackBinding) -> String {
    domain_hash(b"deve.local-repo-removal.fallback.v1\0", binding.as_str())
}

fn domain_hash(domain: &[u8], value: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(domain);
    hash.update(value.as_bytes());
    format!("{:x}", hash.finalize())
}

fn optional_hash_eq(expected: Option<&str>, supplied: Option<&str>) -> bool {
    match (expected, supplied) {
        (None, None) => true,
        (Some(expected), Some(supplied)) => {
            constant_time_eq(expected.as_bytes(), supplied.as_bytes())
        }
        _ => false,
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}
