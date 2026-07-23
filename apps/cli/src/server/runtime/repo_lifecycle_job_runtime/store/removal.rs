//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! Same-owner durable removal preparation and admitted-job records.

mod execution;
mod persistence;
mod repair;
mod retention;

pub(crate) use execution::{
    RemovalCleanupDisposition, RemovalCleanupReceipt, RemovalCleanupStep, RemovalCutState,
    RemovalExecutionState, RemovalTerminalState,
};
#[cfg(test)]
pub(super) use retention::removal_retention_removals_for_test;

use super::{LifecycleReceipt, store_invalid};
use crate::server::runtime::repo_lifecycle_job_runtime::removal::{
    RepoRemovalFallbackSnapshot, RepoRemovalIssuerBinding, RepoRemovalManifest,
    RepoRemovalRepairIssuerBinding,
};
use deve_core::models::RepoId;
use deve_core::protocol::LocalRepoRemovalPreview;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ReceiptStore;

const FORMAT: &str = "deve.host-local-repo-removal";
const VERSION: u32 = 4;
#[cfg(test)]
pub(crate) const PRE_REPLACE_FAILURE_MARKER: &str = ".inject-removal-pre-replace-failure";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum RemovalPreparationState {
    Prepared {
        token_hash: Option<String>,
        fallback_binding_hash: Option<String>,
    },
    Superseded,
    ExecuteAdmitted {
        execute_request_id: Uuid,
        consumed_token_hash: String,
        consumed_fallback_hash: Option<String>,
        switch_nonce: u64,
        receipt: Box<LifecycleReceipt>,
        execution: Box<RemovalExecutionState>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RemovalRepairAuthorization {
    token_hash: String,
    inspection_digest: String,
    execution_digest: String,
    issuer: RepoRemovalRepairIssuerBinding,
    expires_at_unix_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RemovalRepairConsumption {
    token_hash: String,
    issuer: RepoRemovalRepairIssuerBinding,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RemovalPreparationRecord {
    format: String,
    version: u32,
    pub(crate) prepare_request_id: Uuid,
    pub(crate) preparation_id: Uuid,
    pub(crate) repo_id: RepoId,
    pub(crate) scope_nonce: u64,
    pub(crate) fallback_repo_id: Option<RepoId>,
    pub(crate) issuer: RepoRemovalIssuerBinding,
    pub(crate) runtime_incarnation: Uuid,
    pub(crate) manifest_digest: Option<String>,
    pub(crate) manifest: Option<RepoRemovalManifest>,
    pub(crate) preview: LocalRepoRemovalPreview,
    pub(crate) fallback: Option<RepoRemovalFallbackSnapshot>,
    pub(crate) expires_at_unix_ms: i64,
    pub(crate) state: RemovalPreparationState,
    pub(crate) repair_authorization: Option<RemovalRepairAuthorization>,
    pub(crate) repair_consumption: Option<RemovalRepairConsumption>,
    pub(crate) updated_at_ms: i64,
}

impl RemovalPreparationRecord {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepared(
        prepare_request_id: Uuid,
        preparation_id: Uuid,
        repo_id: RepoId,
        scope_nonce: u64,
        fallback_repo_id: Option<RepoId>,
        issuer: RepoRemovalIssuerBinding,
        runtime_incarnation: Uuid,
        manifest_digest: Option<String>,
        manifest: Option<RepoRemovalManifest>,
        preview: LocalRepoRemovalPreview,
        token_hash: Option<String>,
        fallback_binding_hash: Option<String>,
        expires_at_unix_ms: i64,
    ) -> Self {
        let fallback = manifest
            .as_ref()
            .and_then(|manifest| manifest.fallback.clone());
        Self {
            format: FORMAT.to_owned(),
            version: VERSION,
            prepare_request_id,
            preparation_id,
            repo_id,
            scope_nonce,
            fallback_repo_id,
            issuer,
            runtime_incarnation,
            manifest_digest,
            manifest,
            preview,
            fallback,
            expires_at_unix_ms,
            state: RemovalPreparationState::Prepared {
                token_hash,
                fallback_binding_hash,
            },
            repair_authorization: None,
            repair_consumption: None,
            updated_at_ms: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub(super) fn receipt(&self) -> Option<&LifecycleReceipt> {
        match &self.state {
            RemovalPreparationState::ExecuteAdmitted { receipt, .. } => Some(receipt.as_ref()),
            _ => None,
        }
    }

    pub(super) fn has_committed_debt(&self) -> bool {
        matches!(
            &self.state,
            RemovalPreparationState::ExecuteAdmitted { execution, .. }
                if execution.has_committed_debt()
        )
    }

    pub(crate) fn receipt_for_request(&self, request_id: Uuid) -> Option<&LifecycleReceipt> {
        match &self.state {
            RemovalPreparationState::ExecuteAdmitted {
                execute_request_id,
                receipt,
                ..
            } if *execute_request_id == request_id => Some(receipt.as_ref()),
            _ => None,
        }
    }

    pub(super) fn receipt_mut_for_request(
        &mut self,
        request_id: Uuid,
    ) -> Option<&mut LifecycleReceipt> {
        match &mut self.state {
            RemovalPreparationState::ExecuteAdmitted {
                execute_request_id,
                receipt,
                ..
            } if *execute_request_id == request_id => Some(receipt.as_mut()),
            _ => None,
        }
    }

    pub(super) fn validate(&self) -> Result<(), super::super::RepoLifecycleJobError> {
        if self.format != FORMAT
            || self.version != VERSION
            || self.prepare_request_id.is_nil()
            || self.preparation_id.is_nil()
            || self.prepare_request_id == self.preparation_id
        {
            return Err(store_invalid("invalid removal preparation identity"));
        }
        match (&self.manifest, &self.manifest_digest) {
            (Some(manifest), Some(digest))
                if self.repo_id == manifest.repo_id
                    && self.fallback.as_ref() == manifest.fallback.as_ref()
                    && *digest == manifest_digest(manifest)? =>
            {
                validate_hash(digest)?;
            }
            (None, None) if !self.preview.blockers.is_empty() => {}
            _ => return Err(store_invalid("removal preparation manifest mismatch")),
        }
        match &self.state {
            RemovalPreparationState::Prepared {
                token_hash,
                fallback_binding_hash,
            } => {
                if let Some(hash) = token_hash {
                    validate_hash(hash)?;
                }
                if self.preview.blockers.is_empty()
                    != (token_hash.is_some() && self.manifest.is_some())
                {
                    return Err(store_invalid("removal token/blocker state mismatch"));
                }
                if let Some(hash) = fallback_binding_hash {
                    validate_hash(hash)?;
                }
            }
            RemovalPreparationState::Superseded => {}
            RemovalPreparationState::ExecuteAdmitted {
                execute_request_id,
                consumed_token_hash,
                consumed_fallback_hash,
                receipt,
                execution,
                ..
            } => {
                if execute_request_id.is_nil()
                    || *execute_request_id == self.prepare_request_id
                    || *execute_request_id == self.preparation_id
                {
                    return Err(store_invalid(
                        "removal prepare and execute request ids must be distinct",
                    ));
                }
                validate_hash(consumed_token_hash)?;
                if let Some(hash) = consumed_fallback_hash {
                    validate_hash(hash)?;
                }
                if *execute_request_id != receipt.request_id
                    || receipt.target_repo_id != self.repo_id
                    || receipt.operation != super::super::RepoLifecycleJobOperation::Remove
                {
                    return Err(store_invalid("admitted removal receipt identity mismatch"));
                }
                receipt.validate(*execute_request_id)?;
                let digest = self
                    .manifest_digest
                    .as_deref()
                    .ok_or_else(|| store_invalid("admitted removal has no manifest digest"))?;
                execution.validate(*execute_request_id, digest, receipt)?;
            }
        }
        if self.repair_authorization.is_some() && self.repair_consumption.is_some() {
            return Err(store_invalid(
                "removal repair authorization and consumption overlap",
            ));
        }
        match (&self.state, &self.repair_authorization) {
            (RemovalPreparationState::ExecuteAdmitted { execution, .. }, Some(authorization)) => {
                validate_hash(&authorization.token_hash)?;
                validate_hash(&authorization.inspection_digest)?;
                validate_hash(&authorization.execution_digest)?;
                authorization.issuer.validate()?;
                if authorization.expires_at_unix_ms <= 0
                    || authorization.execution_digest != execution_digest(execution)?
                    || !execution.has_committed_debt()
                {
                    return Err(store_invalid("invalid removal repair authorization"));
                }
            }
            (RemovalPreparationState::ExecuteAdmitted { .. }, None)
            | (RemovalPreparationState::Prepared { .. }, None)
            | (RemovalPreparationState::Superseded, None) => {}
            _ => {
                return Err(store_invalid(
                    "removal repair authorization is not bound to admitted cleanup debt",
                ));
            }
        }
        match (&self.state, &self.repair_consumption) {
            (RemovalPreparationState::ExecuteAdmitted { .. }, Some(consumption)) => {
                validate_hash(&consumption.token_hash)?;
                consumption.issuer.validate()?;
            }
            (RemovalPreparationState::ExecuteAdmitted { .. }, None)
            | (RemovalPreparationState::Prepared { .. }, None)
            | (RemovalPreparationState::Superseded, None) => {}
            _ => {
                return Err(store_invalid(
                    "removal repair consumption is not bound to admitted cleanup",
                ));
            }
        }
        Ok(())
    }
}

impl ReceiptStore {
    pub(crate) fn removal_by_prepare_request(
        &self,
        request_id: Uuid,
    ) -> Option<&RemovalPreparationRecord> {
        self.removals
            .values()
            .find(|record| record.prepare_request_id == request_id)
    }

    pub(crate) fn removal(&self, preparation_id: Uuid) -> Option<&RemovalPreparationRecord> {
        self.removals.get(&preparation_id)
    }

    pub(crate) fn removal_by_execute_request(
        &self,
        request_id: Uuid,
    ) -> Option<&RemovalPreparationRecord> {
        self.removals
            .values()
            .find(|record| record.receipt_for_request(request_id).is_some())
    }

    pub(crate) fn removal_has_committed_debt_for_request(&self, request_id: Uuid) -> bool {
        self.removal_by_execute_request(request_id)
            .is_some_and(RemovalPreparationRecord::has_committed_debt)
    }

    pub(crate) fn publish_preparation(
        &mut self,
        record: RemovalPreparationRecord,
    ) -> Result<(), super::super::RepoLifecycleJobError> {
        if self.request_id_is_bound_outside_preparation(
            record.prepare_request_id,
            record.preparation_id,
        ) {
            return Err(super::super::RepoLifecycleJobError::RequestConflict);
        }
        let superseded = self
            .removals
            .iter()
            .filter_map(|(id, existing)| {
                (*id != record.preparation_id
                    && existing.repo_id == record.repo_id
                    && matches!(existing.state, RemovalPreparationState::Prepared { .. }))
                .then_some(*id)
            })
            .collect::<Vec<_>>();
        for id in superseded {
            let mut existing = self
                .removals
                .get(&id)
                .cloned()
                .ok_or_else(|| store_invalid("removal preparation disappeared"))?;
            existing.state = RemovalPreparationState::Superseded;
            existing.updated_at_ms = chrono::Utc::now().timestamp_millis();
            publish_removal(&self.removal_dir, &existing)?;
            self.removals.insert(id, existing);
        }
        record.validate()?;
        publish_removal(&self.removal_dir, &record)?;
        self.removals.insert(record.preparation_id, record);
        Ok(())
    }

    pub(crate) fn admit_prepared_removal(
        &mut self,
        preparation_id: Uuid,
        execute_request_id: Uuid,
        consumed_token_hash: String,
        consumed_fallback_hash: Option<String>,
        switch_nonce: u64,
        receipt: LifecycleReceipt,
    ) -> Result<LifecycleReceipt, super::super::RepoLifecycleJobError> {
        if execute_request_id.is_nil()
            || self.request_id_is_bound(execute_request_id)
            || self
                .removals
                .get(&preparation_id)
                .is_some_and(|record| record.prepare_request_id == execute_request_id)
        {
            return Err(super::super::RepoLifecycleJobError::RequestConflict);
        }
        let mut record = self
            .removals
            .get(&preparation_id)
            .cloned()
            .ok_or(super::super::RepoLifecycleJobError::NotFound)?;
        if !matches!(record.state, RemovalPreparationState::Prepared { .. }) {
            return Err(super::super::RepoLifecycleJobError::ConfirmationInvalid);
        }
        record.state = RemovalPreparationState::ExecuteAdmitted {
            execute_request_id,
            consumed_token_hash,
            consumed_fallback_hash,
            switch_nonce,
            receipt: Box::new(receipt.clone()),
            execution: Box::new(RemovalExecutionState::default()),
        };
        record.repair_authorization = None;
        record.repair_consumption = None;
        record.updated_at_ms = chrono::Utc::now().timestamp_millis();
        record.validate()?;
        publish_removal(&self.removal_dir, &record)?;
        self.removals.insert(preparation_id, record);
        Ok(receipt)
    }

    pub(crate) fn update_removal_execution(
        &mut self,
        preparation_id: Uuid,
        execute_request_id: Uuid,
        mutate: impl FnOnce(
            &mut RemovalExecutionState,
            &mut LifecycleReceipt,
        ) -> Result<(), super::super::RepoLifecycleJobError>,
    ) -> Result<RemovalExecutionState, super::super::RepoLifecycleJobError> {
        let mut record = self
            .removals
            .get(&preparation_id)
            .cloned()
            .ok_or(super::super::RepoLifecycleJobError::NotFound)?;
        let RemovalPreparationState::ExecuteAdmitted {
            execute_request_id: stored_request_id,
            receipt,
            execution,
            ..
        } = &mut record.state
        else {
            return Err(super::super::RepoLifecycleJobError::ConfirmationInvalid);
        };
        if *stored_request_id != execute_request_id {
            return Err(super::super::RepoLifecycleJobError::RequestConflict);
        }
        mutate(execution, receipt)?;
        record.repair_authorization = None;
        record.updated_at_ms = chrono::Utc::now().timestamp_millis();
        record.validate()?;
        publish_removal(&self.removal_dir, &record)?;
        let execution = match &record.state {
            RemovalPreparationState::ExecuteAdmitted { execution, .. } => {
                execution.as_ref().clone()
            }
            _ => unreachable!("validated admitted removal changed variant"),
        };
        self.removals.insert(preparation_id, record);
        Ok(execution)
    }
}

pub(crate) fn manifest_digest(
    manifest: &RepoRemovalManifest,
) -> Result<String, super::super::RepoLifecycleJobError> {
    use sha2::{Digest, Sha256};
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(manifest)?)
    ))
}

pub(super) use persistence::{load_removals, publish_removal};
use repair::execution_digest;
pub(super) use retention::prune_removals;

fn validate_hash(value: &str) -> Result<(), super::super::RepoLifecycleJobError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(store_invalid("removal record hash is malformed"))
    }
}
