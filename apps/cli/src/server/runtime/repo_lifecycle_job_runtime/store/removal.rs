//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! Same-owner durable removal preparation and admitted-job records.

use super::{
    LifecycleReceipt, RECEIPT_MAX_BYTES, TERMINAL_RECEIPT_LIMIT, TERMINAL_RETENTION_MS,
    checked_directory, is_reparse, store_invalid,
};
use crate::server::runtime::repo_lifecycle_job_runtime::removal::{
    RepoRemovalFallbackSnapshot, RepoRemovalIssuerBinding, RepoRemovalManifest,
};
use deve_core::models::RepoId;
use deve_core::protocol::LocalRepoRemovalPreview;
use deve_core::utils::fs as safe_fs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;
use uuid::Uuid;

use super::ReceiptStore;

const FORMAT: &str = "deve.host-local-repo-removal";
const VERSION: u32 = 1;
const STORE_ENTRY_LIMIT: usize = 2_048;
const STORE_AGGREGATE_MAX_BYTES: u64 = 16 * 1024 * 1024;

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
    },
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
            updated_at_ms: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub(super) fn receipt(&self) -> Option<&LifecycleReceipt> {
        match &self.state {
            RemovalPreparationState::ExecuteAdmitted { receipt, .. } => Some(receipt.as_ref()),
            _ => None,
        }
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
                ..
            } => {
                if execute_request_id.is_nil() || *execute_request_id == self.prepare_request_id {
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
        };
        record.updated_at_ms = chrono::Utc::now().timestamp_millis();
        record.validate()?;
        publish_removal(&self.removal_dir, &record)?;
        self.removals.insert(preparation_id, record);
        Ok(receipt)
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

pub(super) fn load_removals(
    dir: &Path,
) -> Result<BTreeMap<Uuid, RemovalPreparationRecord>, super::super::RepoLifecycleJobError> {
    let dir = checked_directory(dir, true)?;
    let mut records = BTreeMap::new();
    let mut entry_count = 0_usize;
    let mut aggregate_bytes = 0_u64;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        entry_count = entry_count.saturating_add(1);
        aggregate_bytes = aggregate_bytes.saturating_add(metadata.len());
        if entry_count > STORE_ENTRY_LIMIT || aggregate_bytes > STORE_AGGREGATE_MAX_BYTES {
            return Err(store_invalid("removal store exceeds bounded load budget"));
        }
        if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
            return Err(store_invalid("removal store contains a non-regular entry"));
        }
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let preparation_id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .and_then(|value| Uuid::parse_str(value).ok())
            .ok_or_else(|| store_invalid("removal record name is not a preparation UUID"))?;
        let record = read_removal(&path)?;
        if record.preparation_id != preparation_id
            || records.insert(preparation_id, record).is_some()
        {
            return Err(store_invalid("duplicate or mismatched removal preparation"));
        }
    }
    Ok(records)
}

pub(super) fn publish_removal(
    dir: &Path,
    record: &RemovalPreparationRecord,
) -> Result<(), super::super::RepoLifecycleJobError> {
    record.validate()?;
    let path = dir.join(format!("{}.json", record.preparation_id));
    let temp = dir.join(format!(
        ".{}.{}.{}.tmp",
        record.preparation_id,
        std::process::id(),
        Uuid::new_v4()
    ));
    let mut bytes = serde_json::to_vec_pretty(record)?;
    bytes.push(b'\n');
    if bytes.len() as u64 > RECEIPT_MAX_BYTES {
        return Err(store_invalid("removal preparation exceeds size budget"));
    }
    let result = (|| -> Result<(), super::super::RepoLifecycleJobError> {
        let mut file = safe_fs::create_atomic_replace_temp(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        safe_fs::replace_file_atomically(&file, &temp, &path)?;
        safe_fs::sync_directory(dir)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

pub(super) fn prune_removals(
    dir: &Path,
    records: &mut BTreeMap<Uuid, RemovalPreparationRecord>,
    now_ms: i64,
) -> Result<usize, super::super::RepoLifecycleJobError> {
    let remove = removal_retention_removals(records, now_ms);
    for id in &remove {
        let path = dir.join(format!("{id}.json"));
        let metadata = std::fs::symlink_metadata(&path)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
            return Err(store_invalid(
                "refusing to prune a non-regular removal record",
            ));
        }
        std::fs::remove_file(path)?;
        records.remove(id);
    }
    if !remove.is_empty() {
        safe_fs::sync_directory(dir)?;
    }
    Ok(remove.len())
}

fn removal_retention_removals(
    records: &BTreeMap<Uuid, RemovalPreparationRecord>,
    now_ms: i64,
) -> Vec<Uuid> {
    let cutoff = now_ms.saturating_sub(TERMINAL_RETENTION_MS);
    let mut candidates = records
        .values()
        .filter(|record| match &record.state {
            RemovalPreparationState::Prepared { .. } => record.expires_at_unix_ms < now_ms,
            RemovalPreparationState::Superseded => true,
            RemovalPreparationState::ExecuteAdmitted { receipt, .. } => {
                receipt.phase.is_terminal() && !receipt.publication_pending
            }
        })
        .map(|record| (record.preparation_id, record.updated_at_ms))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    candidates
        .into_iter()
        .enumerate()
        .filter_map(|(index, (id, updated_at_ms))| {
            (index >= TERMINAL_RECEIPT_LIMIT || updated_at_ms < cutoff).then_some(id)
        })
        .collect()
}

fn read_removal(
    path: &Path,
) -> Result<RemovalPreparationRecord, super::super::RepoLifecycleJobError> {
    let file = safe_fs::open_regular_file_read(path, "repo removal preparation")?;
    let metadata = file.metadata()?;
    if metadata.len() > RECEIPT_MAX_BYTES {
        return Err(store_invalid("removal preparation exceeds size budget"));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(RECEIPT_MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > RECEIPT_MAX_BYTES {
        return Err(store_invalid(
            "removal preparation exceeds read size budget",
        ));
    }
    let record: RemovalPreparationRecord = serde_json::from_slice(&bytes)?;
    record.validate()?;
    Ok(record)
}

#[cfg(test)]
pub(super) fn removal_retention_removals_for_test(
    records: &BTreeMap<Uuid, RemovalPreparationRecord>,
    now_ms: i64,
) -> Vec<Uuid> {
    removal_retention_removals(records, now_ms)
}

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
