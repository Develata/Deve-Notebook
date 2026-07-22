//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 04_repository#local-repo-removal-contract
//!
//! Bounded lifecycle receipt model and identity validation.

use super::store_invalid;
use crate::server::runtime::repo_lifecycle_job_runtime::{
    RepoLifecycleJobCompletion, RepoLifecycleJobError, RepoLifecycleJobIntent,
    RepoLifecycleJobOperation, RepoLifecycleJobOutcome, RepoLifecycleJobPhase,
    RepoLifecycleJobStatus, RepoLifecycleSettledPublication,
};
use deve_core::models::RepoId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const RECEIPT_FORMAT: &str = "deve.host-repo-lifecycle-job";
const RECEIPT_VERSION: u32 = 1;
const PRIMARY_MAX_BYTES: usize = 2 * 1024;
const CLEANUP_MAX_ITEMS: usize = 8;
const CLEANUP_ITEM_MAX_BYTES: usize = 1024;
const PUBLICATION_ERROR_MAX_BYTES: usize = 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::server::runtime::repo_lifecycle_job_runtime) struct LifecycleReceipt {
    format: String,
    version: u32,
    pub(in crate::server::runtime::repo_lifecycle_job_runtime) request_id: Uuid,
    pub(in crate::server::runtime::repo_lifecycle_job_runtime) job_id: Uuid,
    pub(in crate::server::runtime::repo_lifecycle_job_runtime) target_repo_id: RepoId,
    pub(in crate::server::runtime::repo_lifecycle_job_runtime) operation: RepoLifecycleJobOperation,
    intent_digest: String,
    pub(in crate::server::runtime::repo_lifecycle_job_runtime) intent: RepoLifecycleJobIntent,
    pub(in crate::server::runtime::repo_lifecycle_job_runtime) phase: RepoLifecycleJobPhase,
    pub(in crate::server::runtime::repo_lifecycle_job_runtime) outcome:
        Option<RepoLifecycleJobOutcome>,
    pub(in crate::server::runtime::repo_lifecycle_job_runtime) publication:
        Option<RepoLifecycleSettledPublication>,
    pub(in crate::server::runtime::repo_lifecycle_job_runtime) publication_pending: bool,
    pub(in crate::server::runtime::repo_lifecycle_job_runtime) publication_attempts: u32,
    pub(in crate::server::runtime::repo_lifecycle_job_runtime) publication_last_error:
        Option<String>,
    pub(in crate::server::runtime::repo_lifecycle_job_runtime) primary: Option<String>,
    pub(in crate::server::runtime::repo_lifecycle_job_runtime) cleanup: Vec<String>,
    pub(in crate::server::runtime::repo_lifecycle_job_runtime) admitted_at_ms: i64,
    pub(in crate::server::runtime::repo_lifecycle_job_runtime) updated_at_ms: i64,
}

impl LifecycleReceipt {
    pub(in crate::server::runtime::repo_lifecycle_job_runtime) fn admitted(
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

    pub(in crate::server::runtime::repo_lifecycle_job_runtime) fn status(
        &self,
    ) -> RepoLifecycleJobStatus {
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

    pub(in crate::server::runtime::repo_lifecycle_job_runtime) fn matches_intent(
        &self,
        intent: &RepoLifecycleJobIntent,
    ) -> Result<bool, RepoLifecycleJobError> {
        Ok(self.intent_digest == intent_digest(intent)? && self.intent == *intent)
    }

    pub(in crate::server::runtime::repo_lifecycle_job_runtime) fn mark_recovering(&mut self) {
        self.phase = RepoLifecycleJobPhase::Recovering;
        self.outcome = None;
        self.publication = None;
        self.publication_pending = false;
        self.primary = None;
        self.cleanup.clear();
        self.updated_at_ms = chrono::Utc::now().timestamp_millis();
    }

    pub(in crate::server::runtime::repo_lifecycle_job_runtime) fn complete(
        &mut self,
        completion: RepoLifecycleJobCompletion,
    ) {
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

    pub(in crate::server::runtime::repo_lifecycle_job_runtime) fn mark_publication_delivered(
        &mut self,
    ) {
        self.publication_pending = false;
        self.publication_last_error = None;
        self.updated_at_ms = chrono::Utc::now().timestamp_millis();
    }

    pub(in crate::server::runtime::repo_lifecycle_job_runtime) fn append_publication_failure(
        &mut self,
        diagnostic: String,
    ) {
        self.publication_attempts = self.publication_attempts.saturating_add(1);
        self.publication_last_error = Some(truncate_utf8(diagnostic, PUBLICATION_ERROR_MAX_BYTES));
        self.updated_at_ms = chrono::Utc::now().timestamp_millis();
    }

    pub(super) fn validate(&self, path_request_id: Uuid) -> Result<(), RepoLifecycleJobError> {
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
