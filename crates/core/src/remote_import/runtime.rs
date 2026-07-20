//! plan_ref:
//!   - 06_backup#remote-import-runtime-boundary
//!   - 06_backup#remote-import-state-machine
//!   - 03_storage/repair#remote-import-cleanup-repair

mod authority;
mod capture;
mod refresh;

use super::apply::PreparedRemoteImportApply;
use super::apply::{settle_projection_degraded, settle_projection_written};
use super::artifact::{
    ArtifactCapture, RemoteImportArtifactRoot, verify_apply_artifacts,
    verify_exact_published_session,
};
use super::error::{RemoteImportError, RemoteImportResult};
use super::repair::{RemoteImportRepairReport, dry_run_repair};
use super::store::RemoteImportStore;
use super::types::{
    RemoteImportApplyReceipt, RemoteImportApplyRequest, RemoteImportCandidateRevision,
    RemoteImportFailure, RemoteImportFailurePhase, RemoteImportPrepareRequest,
    RemoteImportProjectionOutcome, RemoteImportSessionId, RemoteImportSessionRecord,
    RemoteImportState,
};
use crate::{ledger::RepoManager, models::RepoId};
use authority::bound_local_authority_db;
pub(crate) use capture::RemoteImportCapture;
use capture::{classify_failure, fail_with_primary};
use redb::Database;
use std::path::Path;
use std::sync::Arc;

pub(crate) struct RemoteImportRuntime {
    pub(super) store: RemoteImportStore,
    pub(super) artifacts: RemoteImportArtifactRoot,
}

impl RemoteImportRuntime {
    pub(crate) fn open(repo: &RepoManager, repo_id: RepoId) -> RemoteImportResult<Self> {
        let db = bound_local_authority_db(repo, repo_id)?;
        Self::open_bound(db, repo.ledger_dir(), repo_id, false)
    }

    pub(crate) fn recover_startup(repo: &RepoManager, repo_id: RepoId) -> RemoteImportResult<()> {
        let db = bound_local_authority_db(repo, repo_id)?;
        Self::open_bound(db, repo.ledger_dir(), repo_id, true).map(|_| ())
    }

    #[cfg(test)]
    pub(crate) fn open_for_test(
        db: Arc<Database>,
        ledger_root: &Path,
        repo_id: RepoId,
    ) -> RemoteImportResult<Self> {
        Self::open_bound(db, ledger_root, repo_id, false)
    }

    fn open_bound(
        db: Arc<Database>,
        ledger_root: &Path,
        repo_id: RepoId,
        recover_startup: bool,
    ) -> RemoteImportResult<Self> {
        RemoteImportStore::validate_schema(db.as_ref())?;
        let info = RepoManager::read_local_repo_info_from_db(db.as_ref())
            .map_err(RemoteImportError::storage)?
            .ok_or_else(|| {
                RemoteImportError::Storage(
                    "Remote Import local authority RepoInfo is missing".to_string(),
                )
            })?;
        if info.uuid != repo_id {
            return Err(RemoteImportError::Storage(format!(
                "Remote Import RepoId {} does not match local authority RepoId {}",
                repo_id, info.uuid
            )));
        }
        let artifacts = RemoteImportArtifactRoot::open(ledger_root, repo_id)?;
        let store = if recover_startup {
            RemoteImportStore::recover_startup(db, repo_id)?
        } else {
            RemoteImportStore::open(db, repo_id)?
        };
        Ok(Self { store, artifacts })
    }

    pub(crate) fn dry_run_repair(
        repo: &RepoManager,
        repo_id: RepoId,
    ) -> RemoteImportResult<RemoteImportRepairReport> {
        let db = bound_local_authority_db(repo, repo_id)?;
        let store = RemoteImportStore::open_read_only(db, repo_id)?;
        let artifacts = RemoteImportArtifactRoot::open_existing(repo.ledger_dir(), repo_id)?;
        dry_run_repair(&store, artifacts.as_ref())
    }

    pub(crate) fn begin_prepare(
        &self,
        request: RemoteImportPrepareRequest,
    ) -> RemoteImportResult<RemoteImportCapture> {
        let record = self.store.reserve(
            request.source_binding_digest,
            request.locator_binding_digest,
            request.baseline.ledger_head,
            request.baseline.ignore_digest,
        )?;
        let capture = match ArtifactCapture::start(
            self.artifacts.clone(),
            record.session_id,
            record.generation,
        ) {
            Ok(capture) => capture,
            Err(error) => {
                return Err(fail_with_primary(
                    &self.store,
                    &record,
                    RemoteImportFailure {
                        phase: RemoteImportFailurePhase::Capture,
                        kind: classify_failure(&error),
                    },
                    error,
                )
                .0);
            }
        };
        Ok(RemoteImportCapture {
            store: self.store.clone(),
            record,
            baseline: request.baseline,
            capture: Some(capture),
            pending_failure: None,
            settled: false,
        })
    }

    pub(crate) fn session(
        &self,
        session_id: RemoteImportSessionId,
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        self.store.read_session(session_id)
    }

    #[allow(dead_code)] // B1 store surface; B4 wires the typed List product request.
    pub(crate) fn sessions(&self) -> RemoteImportResult<Vec<RemoteImportSessionRecord>> {
        self.store.list_sessions()
    }

    pub(super) fn repo_removal_observation(
        &self,
    ) -> RemoteImportResult<(u64, Vec<RemoteImportSessionRecord>)> {
        self.store.repo_removal_observation()
    }

    pub(crate) fn prepare_apply(
        &self,
        repo: &RepoManager,
        repo_name: &str,
        request: RemoteImportApplyRequest,
    ) -> RemoteImportResult<crate::ledger::manager::prepared_change_batch::PreparedLedgerChangeBatch>
    {
        let record = self.store.read_session(request.session_id)?;
        let prepared = match record.state {
            RemoteImportState::Ready => {
                let entries = match verify_apply_artifacts(&self.artifacts, &record) {
                    Ok(entries) => entries,
                    Err(error) => {
                        return Err(fail_with_primary(
                            &self.store,
                            &record,
                            RemoteImportFailure {
                                phase: RemoteImportFailurePhase::Verify,
                                kind: classify_failure(&error),
                            },
                            error,
                        )
                        .0);
                    }
                };
                PreparedRemoteImportApply::fresh(
                    record,
                    request,
                    repo.local_peer_id().clone(),
                    entries,
                )?
            }
            RemoteImportState::Applied => {
                PreparedRemoteImportApply::replay(record, request, repo.local_peer_id().clone())?
            }
            _ => {
                return Err(RemoteImportError::InvalidState {
                    session_id: record.session_id,
                    state: record.state,
                    expected: "Ready or same stored Applied request",
                });
            }
        };
        repo.prepare_remote_import_apply_in_local_repo(repo_name, prepared)
    }

    pub(crate) fn commit_apply(
        &self,
        repo: &RepoManager,
        repo_name: &str,
        prepared: crate::ledger::manager::prepared_change_batch::PreparedLedgerChangeBatch,
    ) -> RemoteImportResult<RemoteImportApplyReceipt> {
        repo.commit_prepared_remote_import_apply_in_local_repo(repo_name, prepared)
    }

    pub(crate) fn settle_projection_written(
        &self,
        receipt: &RemoteImportApplyReceipt,
    ) -> RemoteImportResult<RemoteImportApplyReceipt> {
        settle_projection_written(&self.store, receipt)
    }

    pub(crate) fn settle_projection_degraded(
        &self,
        receipt: &RemoteImportApplyReceipt,
        last_error: &str,
    ) -> RemoteImportResult<RemoteImportApplyReceipt> {
        settle_projection_degraded(&self.store, receipt, last_error)
    }

    #[cfg(test)]
    pub(crate) fn finish_cleanup_for_test(
        &self,
        session_id: RemoteImportSessionId,
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        self.store.finish_cleanup(session_id)
    }

    pub(crate) fn discard(
        &self,
        session_id: RemoteImportSessionId,
        expected_revision: Option<RemoteImportCandidateRevision>,
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        let current = self.store.read_session(session_id)?;
        let discarded =
            self.store
                .begin_discard(session_id, current.generation, expected_revision)?;
        if discarded.source_snapshot.is_some() {
            verify_exact_published_session(&self.artifacts, &discarded)?;
        }
        self.artifacts.remove_session_after_inventory(session_id)?;
        self.store.finish_cleanup(discarded.session_id)
    }
}

pub(crate) fn pending_projection_repo_ids(repo: &RepoManager) -> RemoteImportResult<Vec<RepoId>> {
    let mut pending = Vec::new();
    for execution_name in repo
        .list_local_repo_names_for_execution()
        .map_err(RemoteImportError::storage)?
    {
        let handle = repo
            .open_database(None, &execution_name)
            .map_err(RemoteImportError::storage)?;
        let repo_id = handle.repo_id.ok_or_else(|| {
            RemoteImportError::Storage(format!(
                "Remote Import local repo {execution_name} has no RepoId"
            ))
        })?;
        if handle.readonly {
            return Err(RemoteImportError::Storage(format!(
                "Remote Import projection recovery opened local repo {execution_name} read-only"
            )));
        }
        let store = RemoteImportStore::open_read_only(handle.db, repo_id)?;
        if store.list_sessions()?.iter().any(|record| {
            record.state == RemoteImportState::Applied
                && record.apply_receipt.as_ref().is_some_and(|receipt| {
                    receipt.projection_outcome == RemoteImportProjectionOutcome::Pending
                })
        }) {
            pending.push(repo_id);
        }
    }
    pending.sort();
    pending.dedup();
    Ok(pending)
}
