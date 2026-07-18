//! plan_ref:
//!   - 06_backup#remote-import-runtime-boundary
//!   - 06_backup#remote-import-session-contract
//!   - 03_storage/projection#remote-import-projection-writeback
//!
//! Narrow host-facing Remote Import facade. Durable records, artifact paths,
//! digests and the sealed authority batch stay private to `deve_core`.

mod baseline;
mod projection;
mod repair_helpers;
mod review;
mod types;

pub use types::{
    REMOTE_IMPORT_DEFAULT_PAGE_SIZE, REMOTE_IMPORT_MAX_PAGE_SIZE, RemoteImportApplyView,
    RemoteImportBinding, RemoteImportCandidatePage, RemoteImportCandidateView,
    RemoteImportCaptureSink, RemoteImportDiffView, RemoteImportEntryId, RemoteImportPageCursor,
    RemoteImportRepairPlan, RemoteImportSessionView,
};

use super::artifact::verify_apply_artifacts;
use super::repair::{RemoteImportRepairFinding, dry_run_repair};
use super::runtime::RemoteImportRuntime;
use super::types::{
    RemoteImportApplyRequest, RemoteImportPrepareRequest, RemoteImportRefreshRequest,
};
use super::{
    RemoteImportCandidateRevision, RemoteImportError, RemoteImportProjectionOutcome,
    RemoteImportResult, RemoteImportSessionId,
};
use crate::ledger::RepoManager;
use crate::models::RepoId;
use crate::source_control::diff_projection::compute_diff_projection;
use crate::sync::SyncManager;
use baseline::{capture_baseline, ignore_snapshot_digest, projection_locator_digest};
use projection::{settle_pending_projection, writeback_paths};
use repair_helpers::repair_plan;
use review::{
    current_content, dynamic_blockers, ensure_optional_record_revision, ensure_record_repo,
    ensure_record_revision, review_entries,
};
use std::collections::BTreeSet;
use types::{apply_view, candidate_view, cursor_start, display_label, page_cursor, session_view};
use uuid::Uuid;

pub struct RemoteImportService {
    repo_id: RepoId,
    inner: RemoteImportRuntime,
}

impl RemoteImportService {
    pub fn open(repo: &RepoManager, repo_id: RepoId) -> RemoteImportResult<Self> {
        Ok(Self {
            repo_id,
            inner: RemoteImportRuntime::open(repo, repo_id)?,
        })
    }

    pub fn inspect_repair(
        repo: &RepoManager,
        repo_id: RepoId,
    ) -> RemoteImportResult<RemoteImportRepairPlan> {
        repair_plan(RemoteImportRuntime::dry_run_repair(repo, repo_id)?)
    }

    pub fn begin_prepare(
        &self,
        repo: &RepoManager,
        repo_name: &str,
        source_binding: &RemoteImportBinding,
        locator_binding: &RemoteImportBinding,
    ) -> RemoteImportResult<RemoteImportCaptureSink> {
        let baseline = capture_baseline(repo, self.repo_id, repo_name)?;
        let inner = self.inner.begin_prepare(RemoteImportPrepareRequest {
            source_binding_digest: source_binding.digest(),
            locator_binding_digest: locator_binding.digest(),
            baseline,
        })?;
        Ok(RemoteImportCaptureSink { inner })
    }

    pub fn list(&self) -> RemoteImportResult<Vec<RemoteImportSessionView>> {
        self.inner.sessions().map(|records| {
            records
                .iter()
                .map(|record| session_view(record, Vec::new()))
                .collect()
        })
    }

    pub fn show(
        &self,
        repo: &RepoManager,
        repo_name: &str,
        session_id: RemoteImportSessionId,
        expected_revision: Option<RemoteImportCandidateRevision>,
        locator_binding: &RemoteImportBinding,
    ) -> RemoteImportResult<RemoteImportSessionView> {
        let record = self.inner.session(session_id)?;
        ensure_record_repo(&record, self.repo_id)?;
        ensure_optional_record_revision(&record, expected_revision)?;
        let entries = review_entries(&self.inner, &record)?;
        let blockers = dynamic_blockers(repo, repo_name, &record, locator_binding, &entries)?;
        Ok(session_view(&record, blockers))
    }

    #[allow(clippy::too_many_arguments)] // Exact review identity stays explicit at the facade cut.
    pub fn page(
        &self,
        repo: &RepoManager,
        repo_name: &str,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
        cursor: Option<&RemoteImportPageCursor>,
        limit: usize,
        locator_binding: &RemoteImportBinding,
    ) -> RemoteImportResult<RemoteImportCandidatePage> {
        if !(1..=REMOTE_IMPORT_MAX_PAGE_SIZE).contains(&limit) {
            return Err(RemoteImportError::LimitExceeded {
                kind: "candidate page entries",
                limit: REMOTE_IMPORT_MAX_PAGE_SIZE as u64,
                observed: limit as u64,
            });
        }
        let record = self.inner.session(session_id)?;
        ensure_record_revision(&record, self.repo_id, revision)?;
        let entries = review_entries(&self.inner, &record)?;
        let start = cursor_start(session_id, revision, &entries, cursor)?;
        let end = start.saturating_add(limit).min(entries.len());
        let page_entries = entries[start..end]
            .iter()
            .map(candidate_view)
            .collect::<Vec<_>>();
        let next_cursor = (end < entries.len())
            .then(|| page_cursor(session_id, revision, entries[end - 1].entry_id));
        let blockers = dynamic_blockers(repo, repo_name, &record, locator_binding, &entries)?;
        Ok(RemoteImportCandidatePage {
            session: session_view(&record, blockers),
            entries: page_entries,
            next_cursor,
        })
    }

    pub fn diff(
        &self,
        repo: &RepoManager,
        repo_name: &str,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
        entry_id: &RemoteImportEntryId,
    ) -> RemoteImportResult<RemoteImportDiffView> {
        let record = self.inner.session(session_id)?;
        ensure_record_revision(&record, self.repo_id, revision)?;
        let entries = verify_apply_artifacts(&self.inner.artifacts, &record)?;
        let entry = entries
            .into_iter()
            .find(|entry| entry.entry_id.to_hex() == entry_id.as_str())
            .ok_or_else(|| {
                RemoteImportError::ArtifactTampered(
                    "Remote Import entry identity is not part of the candidate".to_string(),
                )
            })?;
        let current = current_content(repo, repo_name, &entry.path)?;
        let projection = compute_diff_projection(current.unwrap_or_default(), entry.content)
            .map_err(RemoteImportError::apply_failed)?;
        Ok(RemoteImportDiffView {
            entry_id: RemoteImportEntryId::from_digest(entry.entry_id),
            display_label: display_label(&entry.path),
            change_kind: entry.change_kind,
            blockers: entry.blockers,
            projection,
        })
    }

    pub fn refresh(
        &self,
        repo: &RepoManager,
        repo_name: &str,
        session_id: RemoteImportSessionId,
        expected_revision: RemoteImportCandidateRevision,
        source_binding: &RemoteImportBinding,
        locator_binding: &RemoteImportBinding,
    ) -> RemoteImportResult<RemoteImportSessionView> {
        let baseline = capture_baseline(repo, self.repo_id, repo_name)?;
        self.inner
            .refresh_from_sealed(
                session_id,
                RemoteImportRefreshRequest {
                    expected_revision,
                    source_binding_digest: source_binding.digest(),
                    locator_binding_digest: locator_binding.digest(),
                    baseline,
                },
            )
            .map(|record| session_view(&record, Vec::new()))
    }

    #[allow(clippy::too_many_arguments)] // Authority dependencies and exact Apply identity must not be hidden in callbacks.
    pub fn apply(
        &self,
        repo: &RepoManager,
        sync: &SyncManager,
        repo_name: &str,
        request_id: Uuid,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
        locator_binding: Option<&RemoteImportBinding>,
    ) -> RemoteImportResult<RemoteImportApplyView> {
        let record = self.inner.session(session_id)?;
        ensure_record_revision(&record, self.repo_id, revision)?;
        if let Some(receipt) = record.apply_receipt.as_ref()
            && receipt.request_id == request_id
            && receipt.revision == revision
        {
            if &receipt.writer_peer_id != repo.local_peer_id() {
                return Err(RemoteImportError::ApplyFailed(
                    "stored Remote Import receipt belongs to a different writer identity"
                        .to_string(),
                ));
            }
            if receipt.projection_outcome == RemoteImportProjectionOutcome::Pending {
                let settled =
                    settle_pending_projection(&self.inner, sync, repo_name, &record, receipt);
                return Ok(apply_view(&settled));
            }
            return Ok(apply_view(receipt));
        }
        let review = review_entries(&self.inner, &record)?;
        let locator_binding = locator_binding.ok_or_else(|| {
            RemoteImportError::ApplyFailed(
                "fresh Remote Import Apply requires an exact remote locator binding".to_string(),
            )
        })?;
        let blockers = dynamic_blockers(repo, repo_name, &record, locator_binding, &review)?;
        if !blockers.is_empty() {
            return Err(RemoteImportError::Blocked {
                session_id,
                blockers,
            });
        }
        let verified = verify_apply_artifacts(&self.inner.artifacts, &record)?;
        let writeback_paths = writeback_paths(&verified);
        let workspace_root = repo
            .local_repo_workspace_root(repo_name)
            .map_err(RemoteImportError::storage)?;
        let ignore_digest = ignore_snapshot_digest(&workspace_root)?;
        let prepared = self.inner.prepare_apply(
            repo,
            repo_name,
            RemoteImportApplyRequest {
                request_id,
                session_id,
                revision,
                locator_digest: projection_locator_digest(repo, self.repo_id, repo_name)?,
                ignore_digest,
            },
        )?;
        let receipt = self.inner.commit_apply(repo, repo_name, prepared)?;
        if receipt.projection_outcome != RemoteImportProjectionOutcome::Pending {
            return Ok(apply_view(&receipt));
        }
        let settled = projection::settle_pending_projection_with_paths(
            &self.inner,
            sync,
            repo_name,
            &receipt,
            &writeback_paths,
        );
        Ok(apply_view(&settled))
    }

    pub fn is_exact_apply_replay(
        &self,
        repo: &RepoManager,
        request_id: Uuid,
        session_id: RemoteImportSessionId,
        revision: RemoteImportCandidateRevision,
    ) -> RemoteImportResult<bool> {
        let record = self.inner.session(session_id)?;
        ensure_record_revision(&record, self.repo_id, revision)?;
        Ok(record.apply_receipt.as_ref().is_some_and(|receipt| {
            receipt.request_id == request_id
                && receipt.revision == revision
                && &receipt.writer_peer_id == repo.local_peer_id()
        }))
    }

    pub(crate) fn recover_pending_projection(
        &self,
        sync: &SyncManager,
        repo_name: &str,
    ) -> RemoteImportResult<()> {
        for record in self.inner.sessions()? {
            let Some(receipt) = record.apply_receipt.as_ref() else {
                continue;
            };
            if receipt.projection_outcome == RemoteImportProjectionOutcome::Pending {
                settle_pending_projection(&self.inner, sync, repo_name, &record, receipt);
            }
        }
        Ok(())
    }

    pub fn discard(
        &self,
        session_id: RemoteImportSessionId,
        expected_revision: Option<RemoteImportCandidateRevision>,
    ) -> RemoteImportResult<RemoteImportSessionView> {
        self.inner
            .discard(session_id, expected_revision)
            .map(|record| session_view(&record, Vec::new()))
    }

    pub fn dry_run_repair(&self) -> RemoteImportResult<RemoteImportRepairPlan> {
        repair_plan(dry_run_repair(
            &self.inner.store,
            Some(&self.inner.artifacts),
        )?)
    }

    /// Executes only cleanup operations whose exact target inventory was
    /// observed by the supplied dry-run token. Authority facts and active
    /// sessions are never changed by this method.
    pub fn apply_repair(&self, expected_token: &str) -> RemoteImportResult<RemoteImportRepairPlan> {
        let observed = dry_run_repair(&self.inner.store, Some(&self.inner.artifacts))?;
        let observed_plan = repair_plan(observed.clone())?;
        if observed_plan.token != expected_token {
            return Err(RemoteImportError::RepairPlanChanged);
        }
        let cleanup_pending = observed
            .findings
            .iter()
            .filter_map(|finding| match finding {
                RemoteImportRepairFinding::CleanupPending(session_id) => Some(*session_id),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let orphan_sessions = observed
            .findings
            .iter()
            .filter_map(|finding| match finding {
                RemoteImportRepairFinding::OrphanSessionArtifact(session_id) => Some(*session_id),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        for session_id in cleanup_pending {
            self.inner
                .artifacts
                .remove_session_after_inventory(session_id)?;
            self.inner.store.finish_cleanup(session_id)?;
        }
        for session_id in orphan_sessions {
            self.inner
                .artifacts
                .remove_session_after_inventory(session_id)?;
        }
        repair_plan(dry_run_repair(
            &self.inner.store,
            Some(&self.inner.artifacts),
        )?)
    }
}

#[cfg(test)]
mod tests;
