//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout
//!   - 04_repository#repo-lifecycle-coordinator

use super::model::{
    RepoLifecycleJobCompletion, RepoLifecycleJobError, RepoLifecycleJobIntent,
    RepoLifecycleJobOperation, RepoLifecycleJobOutcome, RepoLifecycleJobPhase,
    RepoLifecycleJobStatus, RepoLifecycleSettledPublication,
};
use deve_core::models::RepoId;
use deve_core::utils::{fs as safe_fs, notegit};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const RECEIPT_DIR: &str = "repo-lifecycle-jobs";
const LOCK_FILE: &str = "repo-lifecycle-jobs.lock";
const RECEIPT_FORMAT: &str = "deve.host-repo-lifecycle-job";
const RECEIPT_VERSION: u32 = 1;
const RECEIPT_MAX_BYTES: u64 = 64 * 1024;
const TERMINAL_RETENTION_MS: i64 = 24 * 60 * 60 * 1_000;
const TERMINAL_RECEIPT_LIMIT: usize = 1024;
const PRIMARY_MAX_BYTES: usize = 2 * 1024;
const CLEANUP_MAX_ITEMS: usize = 8;
const CLEANUP_ITEM_MAX_BYTES: usize = 1024;
const PUBLICATION_ERROR_MAX_BYTES: usize = 1024;
#[cfg(test)]
pub(super) const POST_REPLACE_FAILURE_MARKER: &str = ".inject-post-replace-failure";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LifecycleReceipt {
    format: String,
    version: u32,
    pub(super) request_id: Uuid,
    pub(super) job_id: Uuid,
    pub(super) target_repo_id: RepoId,
    pub(super) operation: RepoLifecycleJobOperation,
    pub(super) intent_digest: String,
    pub(super) intent: RepoLifecycleJobIntent,
    pub(super) phase: RepoLifecycleJobPhase,
    pub(super) outcome: Option<RepoLifecycleJobOutcome>,
    pub(super) publication: Option<RepoLifecycleSettledPublication>,
    pub(super) publication_pending: bool,
    pub(super) publication_attempts: u32,
    pub(super) publication_last_error: Option<String>,
    pub(super) primary: Option<String>,
    pub(super) cleanup: Vec<String>,
    pub(super) admitted_at_ms: i64,
    pub(super) updated_at_ms: i64,
}

impl LifecycleReceipt {
    pub(super) fn admitted(
        request_id: Uuid,
        job_id: Uuid,
        target_repo_id: RepoId,
        intent: RepoLifecycleJobIntent,
    ) -> Result<Self, RepoLifecycleJobError> {
        let now = chrono::Utc::now().timestamp_millis();
        let intent_digest = intent_digest(&intent)?;
        Ok(Self {
            format: RECEIPT_FORMAT.to_owned(),
            version: RECEIPT_VERSION,
            request_id,
            job_id,
            target_repo_id,
            operation: intent.operation(),
            intent_digest,
            intent,
            phase: RepoLifecycleJobPhase::Running,
            outcome: None,
            publication: None,
            publication_pending: false,
            publication_attempts: 0,
            publication_last_error: None,
            primary: None,
            cleanup: Vec::new(),
            admitted_at_ms: now,
            updated_at_ms: now,
        })
    }

    pub(super) fn status(&self) -> RepoLifecycleJobStatus {
        RepoLifecycleJobStatus {
            request_id: self.request_id,
            job_id: self.job_id,
            target_repo_id: self.target_repo_id,
            operation: self.operation,
            phase: self.phase,
            outcome: self.outcome,
            publication_pending: self.publication_pending,
            publication: self.publication.clone(),
        }
    }

    pub(super) fn matches_intent(
        &self,
        intent: &RepoLifecycleJobIntent,
    ) -> Result<bool, RepoLifecycleJobError> {
        Ok(self.intent_digest == intent_digest(intent)? && self.intent == *intent)
    }

    pub(super) fn mark_phase(&mut self, phase: RepoLifecycleJobPhase) {
        self.phase = phase;
        self.updated_at_ms = chrono::Utc::now().timestamp_millis();
    }

    pub(super) fn complete(&mut self, completion: RepoLifecycleJobCompletion) {
        self.phase = RepoLifecycleJobPhase::Terminal;
        self.outcome = Some(completion.outcome);
        self.publication_pending = completion.publication.is_some();
        self.publication = completion.publication;
        self.primary = completion
            .primary
            .map(|value| truncate_utf8(value, PRIMARY_MAX_BYTES));
        self.cleanup = completion
            .cleanup
            .into_iter()
            .take(CLEANUP_MAX_ITEMS)
            .map(|value| truncate_utf8(value, CLEANUP_ITEM_MAX_BYTES))
            .collect();
        self.updated_at_ms = chrono::Utc::now().timestamp_millis();
    }

    pub(super) fn mark_publication_delivered(&mut self) {
        self.publication_pending = false;
        self.publication_last_error = None;
        self.updated_at_ms = chrono::Utc::now().timestamp_millis();
    }

    pub(super) fn append_publication_failure(&mut self, error: String) {
        self.publication_attempts = self.publication_attempts.saturating_add(1);
        self.publication_last_error = Some(truncate_utf8(error, PUBLICATION_ERROR_MAX_BYTES));
        self.updated_at_ms = chrono::Utc::now().timestamp_millis();
    }

    fn validate(&self, path_request_id: Uuid) -> Result<(), RepoLifecycleJobError> {
        if self.format != RECEIPT_FORMAT || self.version != RECEIPT_VERSION {
            return Err(store_invalid("unsupported receipt format or version"));
        }
        self.intent.validate()?;
        if self.request_id != path_request_id
            || self.intent.operation() != self.operation
            || self.intent_digest != intent_digest(&self.intent)?
        {
            return Err(store_invalid("receipt identity or intent digest mismatch"));
        }
        if self
            .intent
            .requested_repo_id()
            .is_some_and(|id| id != self.target_repo_id)
        {
            return Err(store_invalid("remove target RepoId mismatch"));
        }
        if self.phase.is_terminal() != self.outcome.is_some() {
            return Err(store_invalid("receipt phase/outcome mismatch"));
        }
        if self.publication_pending && self.publication.is_none() {
            return Err(store_invalid("publication debt has no publication payload"));
        }
        if self.publication.is_some()
            && matches!(
                self.outcome,
                Some(
                    RepoLifecycleJobOutcome::NotCommitted | RepoLifecycleJobOutcome::RepairRequired
                )
            )
        {
            return Err(store_invalid(
                "non-committed or repair outcome carries a settled publication",
            ));
        }
        if let Some(publication) = &self.publication {
            let publication_matches = match (self.operation, publication) {
                (
                    RepoLifecycleJobOperation::Create,
                    RepoLifecycleSettledPublication::Created { repo_id, .. },
                )
                | (
                    RepoLifecycleJobOperation::Remove,
                    RepoLifecycleSettledPublication::Removed { repo_id, .. },
                ) => *repo_id == self.target_repo_id,
                _ => false,
            };
            if !publication_matches {
                return Err(store_invalid("publication operation or RepoId mismatch"));
            }
        }
        Ok(())
    }
}

pub(super) struct ReceiptStore {
    dir: PathBuf,
    rows: BTreeMap<Uuid, LifecycleReceipt>,
    _lock: std::fs::File,
}

impl ReceiptStore {
    pub(super) fn open(ledger_dir: &Path) -> Result<Self, RepoLifecycleJobError> {
        let host_dir = checked_directory(&notegit::host_dir(ledger_dir), true)?;
        let lock_path = host_dir.join(LOCK_FILE);
        let lock = safe_fs::open_regular_file_lock(&lock_path, "repo lifecycle job lock")?;
        lock.try_lock().map_err(|error| {
            store_invalid(format!("repo lifecycle job lock is already held: {error}"))
        })?;
        safe_fs::ensure_open_file_matches_path(&lock, &lock_path, "repo lifecycle job lock")?;
        let dir = checked_directory(&host_dir.join(RECEIPT_DIR), true)?;
        let mut rows = BTreeMap::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path)?;
            if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
                return Err(store_invalid(
                    "receipt directory contains a non-regular entry",
                ));
            }
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let request_id = path
                .file_stem()
                .and_then(|value| value.to_str())
                .and_then(|value| Uuid::parse_str(value).ok())
                .ok_or_else(|| store_invalid("receipt file name is not a request UUID"))?;
            let receipt = read_receipt(&path, request_id)?;
            if rows.insert(request_id, receipt).is_some() {
                return Err(store_invalid("duplicate lifecycle request receipt"));
            }
        }
        let mut active_repos = std::collections::HashSet::new();
        for receipt in rows.values().filter(|receipt| !receipt.phase.is_terminal()) {
            if !active_repos.insert(receipt.target_repo_id) {
                return Err(store_invalid(
                    "multiple active receipts target the same RepoId",
                ));
            }
        }
        Ok(Self {
            dir,
            rows,
            _lock: lock,
        })
    }

    pub(super) fn receipt(&self, request_id: Uuid) -> Option<&LifecycleReceipt> {
        self.rows.get(&request_id)
    }

    pub(super) fn active_receipts(&self) -> Vec<LifecycleReceipt> {
        self.rows
            .values()
            .filter(|receipt| !receipt.phase.is_terminal())
            .cloned()
            .collect()
    }

    pub(super) fn pending_publications(&self) -> Vec<Uuid> {
        self.rows
            .values()
            .filter(|receipt| receipt.publication_pending)
            .map(|receipt| receipt.request_id)
            .collect()
    }

    pub(super) fn insert(
        &mut self,
        receipt: LifecycleReceipt,
    ) -> Result<(), RepoLifecycleJobError> {
        if self.rows.contains_key(&receipt.request_id) {
            return Err(store_invalid("duplicate lifecycle request receipt"));
        }
        self.publish(&receipt)?;
        self.rows.insert(receipt.request_id, receipt);
        Ok(())
    }

    pub(super) fn update(
        &mut self,
        request_id: Uuid,
        mutate: impl FnOnce(&mut LifecycleReceipt),
    ) -> Result<LifecycleReceipt, RepoLifecycleJobError> {
        let mut receipt = self
            .rows
            .get(&request_id)
            .cloned()
            .ok_or(RepoLifecycleJobError::NotFound)?;
        mutate(&mut receipt);
        receipt.validate(request_id)?;
        self.publish(&receipt)?;
        self.rows.insert(request_id, receipt.clone());
        Ok(receipt)
    }

    pub(super) fn prune_terminal(
        &mut self,
        mut retain_normal_create: impl FnMut(RepoId) -> bool,
    ) -> Result<usize, RepoLifecycleJobError> {
        let remove = terminal_retention_removals(
            self.rows.values(),
            chrono::Utc::now().timestamp_millis(),
            &mut retain_normal_create,
        );
        for request_id in &remove {
            let path = self.dir.join(format!("{request_id}.json"));
            let metadata = std::fs::symlink_metadata(&path)?;
            if !metadata.is_file() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
                return Err(store_invalid("refusing to prune a non-regular receipt"));
            }
            std::fs::remove_file(path)?;
            self.rows.remove(request_id);
        }
        if !remove.is_empty() {
            safe_fs::sync_directory(&self.dir)?;
        }
        Ok(remove.len())
    }

    fn publish(&self, receipt: &LifecycleReceipt) -> Result<(), RepoLifecycleJobError> {
        let path = self.dir.join(format!("{}.json", receipt.request_id));
        let temp = self.dir.join(format!(
            ".{}.{}.{}.tmp",
            receipt.request_id,
            std::process::id(),
            Uuid::new_v4()
        ));
        let mut bytes = serde_json::to_vec_pretty(receipt)?;
        bytes.push(b'\n');
        if bytes.len() as u64 > RECEIPT_MAX_BYTES {
            return Err(store_invalid("serialized receipt exceeds size budget"));
        }
        let result = (|| -> Result<(), RepoLifecycleJobError> {
            let mut file = safe_fs::create_atomic_replace_temp(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            safe_fs::replace_file_atomically(&file, &temp, &path)?;
            #[cfg(test)]
            if self.dir.join(POST_REPLACE_FAILURE_MARKER).try_exists()? {
                return Err(store_invalid("injected post-replace sync failure"));
            }
            safe_fs::sync_directory(&self.dir)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        result
    }
}

fn terminal_retention_removals<'a>(
    rows: impl Iterator<Item = &'a LifecycleReceipt>,
    now_ms: i64,
    retain_normal_create: &mut impl FnMut(RepoId) -> bool,
) -> Vec<Uuid> {
    let cutoff = now_ms.saturating_sub(TERMINAL_RETENTION_MS);
    let mut candidates = rows
        .filter(|receipt| {
            receipt.phase.is_terminal()
                && !receipt.publication_pending
                && !(receipt.operation == RepoLifecycleJobOperation::Create
                    && retain_normal_create(receipt.target_repo_id))
        })
        .map(|receipt| (receipt.request_id, receipt.updated_at_ms))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    candidates
        .into_iter()
        .enumerate()
        .filter_map(|(index, (request_id, updated_at_ms))| {
            (index >= TERMINAL_RECEIPT_LIMIT || updated_at_ms < cutoff).then_some(request_id)
        })
        .collect()
}

#[cfg(test)]
pub(super) fn retention_removals_for_test(
    receipts: &[LifecycleReceipt],
    now_ms: i64,
    protected: &std::collections::HashSet<RepoId>,
) -> Vec<Uuid> {
    terminal_retention_removals(receipts.iter(), now_ms, &mut |repo_id| {
        protected.contains(&repo_id)
    })
}

fn read_receipt(path: &Path, request_id: Uuid) -> Result<LifecycleReceipt, RepoLifecycleJobError> {
    let file = safe_fs::open_regular_file_read(path, "repo lifecycle receipt")?;
    let metadata = file.metadata()?;
    if metadata.len() > RECEIPT_MAX_BYTES {
        return Err(store_invalid("receipt exceeds size budget"));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(RECEIPT_MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > RECEIPT_MAX_BYTES {
        return Err(store_invalid("receipt exceeds read size budget"));
    }
    let receipt: LifecycleReceipt = serde_json::from_slice(&bytes)?;
    receipt.validate(request_id)?;
    Ok(receipt)
}

fn intent_digest(intent: &RepoLifecycleJobIntent) -> Result<String, RepoLifecycleJobError> {
    let bytes = serde_json::to_vec(intent)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn truncate_utf8(mut value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    let mut boundary = max_bytes;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
    value
}

fn checked_directory(path: &Path, create: bool) -> Result<PathBuf, RepoLifecycleJobError> {
    if create {
        std::fs::create_dir_all(path)?;
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
        return Err(store_invalid(
            "lifecycle runtime path is not a regular directory",
        ));
    }
    Ok(path.to_path_buf())
}

fn store_invalid(detail: impl Into<String>) -> RepoLifecycleJobError {
    RepoLifecycleJobError::Store(detail.into())
}

#[cfg(windows)]
fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse(_metadata: &std::fs::Metadata) -> bool {
    false
}
