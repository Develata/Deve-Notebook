//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! One-shot exact repair authorization for committed cleanup debt.

use super::{
    LifecycleReceipt, ReceiptStore, RemovalExecutionState, RemovalPreparationState,
    RemovalRepairAuthorization, RemovalRepairConsumption, publish_removal,
};
use crate::server::runtime::repo_lifecycle_job_runtime::{
    RepoLifecycleJobError, RepoRemovalRepairIssuerBinding,
};
use uuid::Uuid;

impl ReceiptStore {
    pub(crate) fn publish_removal_repair_authorization(
        &mut self,
        execute_request_id: Uuid,
        token_hash: String,
        inspection_digest: String,
        issuer: RepoRemovalRepairIssuerBinding,
        expires_at_unix_ms: i64,
    ) -> Result<(), RepoLifecycleJobError> {
        let mut record = self
            .removal_by_execute_request(execute_request_id)
            .cloned()
            .ok_or(RepoLifecycleJobError::NotFound)?;
        let execution = match &record.state {
            RemovalPreparationState::ExecuteAdmitted { execution, .. } => execution.as_ref(),
            _ => return Err(RepoLifecycleJobError::RemovalRepairNotRequired),
        };
        if !execution.has_committed_debt() {
            return Err(RepoLifecycleJobError::RemovalRepairNotRequired);
        }
        record.repair_authorization = Some(RemovalRepairAuthorization {
            token_hash,
            inspection_digest,
            execution_digest: execution_digest(execution)?,
            issuer,
            expires_at_unix_ms,
        });
        record.repair_consumption = None;
        record.updated_at_ms = chrono::Utc::now().timestamp_millis();
        record.validate()?;
        publish_removal(&self.removal_dir, &record)?;
        self.removals.insert(record.preparation_id, record);
        Ok(())
    }

    pub(crate) fn clear_removal_repair_authorization(
        &mut self,
        execute_request_id: Uuid,
    ) -> Result<(), RepoLifecycleJobError> {
        let Some(mut record) = self.removal_by_execute_request(execute_request_id).cloned() else {
            return Err(RepoLifecycleJobError::NotFound);
        };
        if record.repair_authorization.is_none() && record.repair_consumption.is_none() {
            return Ok(());
        }
        record.repair_authorization = None;
        record.repair_consumption = None;
        record.updated_at_ms = chrono::Utc::now().timestamp_millis();
        record.validate()?;
        publish_removal(&self.removal_dir, &record)?;
        self.removals.insert(record.preparation_id, record);
        Ok(())
    }

    pub(crate) fn consume_removal_repair_authorization(
        &mut self,
        execute_request_id: Uuid,
        supplied_token_hash: &str,
        inspection_digest: &str,
        issuer: &RepoRemovalRepairIssuerBinding,
        now_ms: i64,
    ) -> Result<LifecycleReceipt, RepoLifecycleJobError> {
        let mut record = self
            .removal_by_execute_request(execute_request_id)
            .cloned()
            .ok_or(RepoLifecycleJobError::NotFound)?;
        let execution = match &record.state {
            RemovalPreparationState::ExecuteAdmitted { execution, .. } => execution.as_ref(),
            _ => return Err(RepoLifecycleJobError::RemovalRepairNotRequired),
        };
        if !execution.has_committed_debt() {
            return Err(RepoLifecycleJobError::RemovalRepairNotRequired);
        }
        let authorization = record
            .repair_authorization
            .as_ref()
            .ok_or(RepoLifecycleJobError::ConfirmationInvalid)?;
        if now_ms >= authorization.expires_at_unix_ms {
            return Err(RepoLifecycleJobError::ConfirmationExpired);
        }
        if !constant_time_eq(
            authorization.token_hash.as_bytes(),
            supplied_token_hash.as_bytes(),
        ) || !constant_time_eq(
            authorization.inspection_digest.as_bytes(),
            inspection_digest.as_bytes(),
        ) || authorization.execution_digest != execution_digest(execution)?
            || &authorization.issuer != issuer
        {
            return Err(RepoLifecycleJobError::ConfirmationStale);
        }
        let consumption = RemovalRepairConsumption {
            token_hash: authorization.token_hash.clone(),
            issuer: authorization.issuer.clone(),
        };
        record.repair_authorization = None;
        record.repair_consumption = Some(consumption);
        let receipt = match &mut record.state {
            RemovalPreparationState::ExecuteAdmitted { receipt, .. } => {
                receipt.mark_recovering();
                receipt.as_ref().clone()
            }
            _ => unreachable!("admitted repair record changed variant"),
        };
        record.updated_at_ms = chrono::Utc::now().timestamp_millis();
        record.validate()?;
        publish_removal(&self.removal_dir, &record)?;
        self.removals.insert(record.preparation_id, record);
        Ok(receipt)
    }

    pub(crate) fn replay_consumed_removal_repair(
        &self,
        execute_request_id: Uuid,
        supplied_token_hash: &str,
        issuer: &RepoRemovalRepairIssuerBinding,
    ) -> Option<LifecycleReceipt> {
        let record = self.removal_by_execute_request(execute_request_id)?;
        let consumption = record.repair_consumption.as_ref()?;
        (constant_time_eq(
            consumption.token_hash.as_bytes(),
            supplied_token_hash.as_bytes(),
        ) && &consumption.issuer == issuer)
            .then(|| record.receipt_for_request(execute_request_id).cloned())
            .flatten()
    }
}

pub(super) fn execution_digest(
    execution: &RemovalExecutionState,
) -> Result<String, RepoLifecycleJobError> {
    use sha2::{Digest, Sha256};
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(execution)?)
    ))
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .fold(0_u8, |difference, (left, right)| {
                difference | (left ^ right)
            })
            == 0
}
