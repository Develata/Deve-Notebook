//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! One request-id namespace across normal lifecycle, removal Prepare, and
//! removal Execute durable records.

use super::{LifecycleReceipt, ReceiptStore, RemovalPreparationRecord, store_invalid};
use crate::server::runtime::repo_lifecycle_job_runtime::model::RepoLifecycleJobError;
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

impl ReceiptStore {
    pub(in crate::server::runtime::repo_lifecycle_job_runtime) fn request_id_is_bound(
        &self,
        request_id: Uuid,
    ) -> bool {
        self.rows.contains_key(&request_id)
            || self.removals.values().any(|record| {
                record.prepare_request_id == request_id
                    || record.receipt_for_request(request_id).is_some()
            })
    }

    pub(in crate::server::runtime::repo_lifecycle_job_runtime) fn request_id_is_bound_outside_preparation(
        &self,
        request_id: Uuid,
        preparation_id: Uuid,
    ) -> bool {
        self.rows.contains_key(&request_id)
            || self.removals.values().any(|record| {
                record.receipt_for_request(request_id).is_some()
                    || (record.preparation_id != preparation_id
                        && record.prepare_request_id == request_id)
            })
    }
}

pub(super) fn validate(
    rows: &BTreeMap<Uuid, LifecycleReceipt>,
    removals: &BTreeMap<Uuid, RemovalPreparationRecord>,
) -> Result<(), RepoLifecycleJobError> {
    let mut bound = HashSet::new();
    for request_id in rows.keys().copied() {
        if request_id.is_nil() || !bound.insert(request_id) {
            return Err(store_invalid("duplicate lifecycle request id"));
        }
    }
    for record in removals.values() {
        if record.prepare_request_id.is_nil() || !bound.insert(record.prepare_request_id) {
            return Err(store_invalid("duplicate removal prepare request id"));
        }
        if let Some(receipt) = record.receipt()
            && (receipt.request_id.is_nil() || !bound.insert(receipt.request_id))
        {
            return Err(store_invalid("duplicate removal execute request id"));
        }
    }
    Ok(())
}
