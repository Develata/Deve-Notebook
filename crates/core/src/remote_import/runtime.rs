//! plan_ref:
//!   - 06_backup#remote-import-runtime-boundary
//!   - 06_backup#remote-import-state-machine
//!   - 03_storage/repair#remote-import-cleanup-repair

mod authority;

use super::artifact::{
    ArtifactCapture, RemoteImportArtifactRoot, publish_candidate_revision,
    verify_exact_published_session, verify_published_session,
};
use super::error::{RemoteImportError, RemoteImportResult};
use super::manifest::encode_candidate;
use super::repair::{RemoteImportRepairReport, dry_run_repair};
use super::store::RemoteImportStore;
use super::types::{
    RemoteImportBaseline, RemoteImportCandidateRevision, RemoteImportFailure,
    RemoteImportFailureKind, RemoteImportFailurePhase, RemoteImportPrepareRequest,
    RemoteImportRefreshRequest, RemoteImportSessionId, RemoteImportSessionRecord,
    RemoteImportState,
};
use crate::{ledger::RepoManager, models::RepoId};
use authority::bound_local_authority_db;
use redb::Database;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

pub(crate) struct RemoteImportRuntime {
    store: RemoteImportStore,
    artifacts: RemoteImportArtifactRoot,
}

impl RemoteImportRuntime {
    pub(crate) fn open(repo: &RepoManager, repo_id: RepoId) -> RemoteImportResult<Self> {
        let db = bound_local_authority_db(repo, repo_id)?;
        Self::open_bound(db, repo.ledger_dir(), repo_id)
    }

    #[cfg(test)]
    pub(crate) fn open_for_test(
        db: Arc<Database>,
        ledger_root: &Path,
        repo_id: RepoId,
    ) -> RemoteImportResult<Self> {
        Self::open_bound(db, ledger_root, repo_id)
    }

    fn open_bound(
        db: Arc<Database>,
        ledger_root: &Path,
        repo_id: RepoId,
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
        let store = RemoteImportStore::open(db, repo_id)?;
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
        if request.locator_binding_digest != request.baseline.locator_digest {
            return Err(RemoteImportError::ArtifactTampered(
                "prepare locator binding does not match captured baseline".to_string(),
            ));
        }
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

    pub(crate) fn refresh_from_sealed(
        &self,
        session_id: RemoteImportSessionId,
        request: RemoteImportRefreshRequest,
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        self.refresh_from_sealed_inner(session_id, request, || {})
    }

    #[cfg(test)]
    pub(crate) fn refresh_with_after_read_test(
        &self,
        session_id: RemoteImportSessionId,
        request: RemoteImportRefreshRequest,
        after_read: impl FnOnce(),
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        self.refresh_from_sealed_inner(session_id, request, after_read)
    }

    fn refresh_from_sealed_inner(
        &self,
        session_id: RemoteImportSessionId,
        request: RemoteImportRefreshRequest,
        after_read: impl FnOnce(),
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        let record = self.store.read_session(session_id)?;
        if !matches!(
            record.state,
            RemoteImportState::Ready | RemoteImportState::Stale
        ) {
            return Err(RemoteImportError::InvalidState {
                session_id,
                state: record.state,
                expected: "Ready or Stale",
            });
        }
        after_read();
        if request.source_binding_digest != record.source_binding_digest {
            self.store.mark_stale(session_id, record.generation)?;
            return Err(RemoteImportError::ArtifactTampered(
                "refresh source/profile binding drifted from sealed session".to_string(),
            ));
        }
        let baseline = request.baseline;
        if baseline.locator_digest != record.locator_binding_digest {
            self.store.mark_stale(session_id, record.generation)?;
            return Err(RemoteImportError::ArtifactTampered(
                "refresh locator binding drifted from sealed session".to_string(),
            ));
        }
        let manifest = match verify_published_session(&self.artifacts, &record) {
            Ok(manifest) => manifest,
            Err(error) => {
                return Err(fail_with_primary(
                    &self.store,
                    &record,
                    RemoteImportFailure {
                        phase: RemoteImportFailurePhase::Verify,
                        kind: RemoteImportFailureKind::DigestMismatch,
                    },
                    error,
                )
                .0);
            }
        };
        let current = record.candidate.as_ref().ok_or_else(|| {
            RemoteImportError::ArtifactTampered("session candidate is missing".to_string())
        })?;
        let revision = current
            .revision
            .next()
            .ok_or(RemoteImportError::RevisionOverflow)?;
        let candidate = encode_candidate(&manifest, &baseline, revision)?;
        if let Err(error) = publish_candidate_revision(&self.artifacts, &record, &candidate) {
            if matches!(error, RemoteImportError::CandidateRevisionConflict { .. }) {
                return Err(error);
            }
            return Err(fail_with_primary(
                &self.store,
                &record,
                RemoteImportFailure {
                    phase: RemoteImportFailurePhase::Publish,
                    kind: classify_failure(&error),
                },
                error,
            )
            .0);
        }
        let mut prospective = record.clone();
        prospective.state = RemoteImportState::Ready;
        prospective.baseline_head = candidate.record.ledger_head;
        prospective.ignore_digest = candidate.record.ignore_digest;
        prospective.candidate = Some(candidate.record.clone());
        prospective.failure = None;
        if let Err(error) = verify_published_session(&self.artifacts, &prospective) {
            return Err(fail_with_primary(
                &self.store,
                &record,
                RemoteImportFailure {
                    phase: RemoteImportFailurePhase::Verify,
                    kind: RemoteImportFailureKind::DigestMismatch,
                },
                error,
            )
            .0);
        }
        match self.store.update_candidate(
            record.session_id,
            record.generation,
            current,
            candidate.record.clone(),
        ) {
            Ok(updated) => Ok(updated),
            Err(primary) => {
                let observed = self.store.read_session(session_id);
                if let Ok(observed) = observed
                    && observed.state == RemoteImportState::Ready
                    && observed.candidate.as_ref() == Some(&candidate.record)
                {
                    return Ok(observed);
                }
                Err(primary)
            }
        }
    }

    pub(crate) fn discard(
        &self,
        session_id: RemoteImportSessionId,
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        let current = self.store.read_session(session_id)?;
        let discarded = self.store.begin_discard(session_id, current.generation)?;
        if discarded.source_snapshot.is_some() {
            verify_exact_published_session(&self.artifacts, &discarded)?;
        }
        self.artifacts.remove_session_after_inventory(session_id)?;
        self.store.finish_cleanup(discarded.session_id)
    }
}

pub(crate) struct RemoteImportCapture {
    store: RemoteImportStore,
    record: RemoteImportSessionRecord,
    baseline: RemoteImportBaseline,
    capture: Option<ArtifactCapture>,
    pending_failure: Option<RemoteImportFailure>,
    settled: bool,
}

impl RemoteImportCapture {
    pub(crate) fn session_id(&self) -> RemoteImportSessionId {
        self.record.session_id
    }

    pub(crate) fn capture_file(&mut self, path: &str, reader: impl Read) -> RemoteImportResult<()> {
        if self.pending_failure.is_some() {
            return Err(RemoteImportError::InvalidState {
                session_id: self.record.session_id,
                state: super::types::RemoteImportState::Failed,
                expected: "Preparing",
            });
        }
        let capture = self
            .capture
            .as_mut()
            .ok_or(RemoteImportError::InvalidState {
                session_id: self.record.session_id,
                state: self.record.state,
                expected: "Preparing",
            })?;
        if let Err(error) = capture.capture_file(path, reader) {
            return Err(self.persist_failure(
                RemoteImportFailurePhase::Capture,
                classify_failure(&error),
                error,
            ));
        }
        Ok(())
    }

    pub(crate) fn finish(mut self) -> RemoteImportResult<RemoteImportSessionRecord> {
        self.finish_inner(|_| Ok(()))
    }

    #[cfg(test)]
    pub(crate) fn finish_with_before_ready_test(
        mut self,
        before_ready: impl FnOnce(&Path) -> RemoteImportResult<()>,
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        self.finish_inner(before_ready)
    }

    fn finish_inner(
        &mut self,
        before_ready: impl FnOnce(&Path) -> RemoteImportResult<()>,
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        if self.pending_failure.is_some() {
            return Err(RemoteImportError::InvalidState {
                session_id: self.record.session_id,
                state: super::types::RemoteImportState::Failed,
                expected: "Preparing",
            });
        }
        let capture = self.capture.take().ok_or(RemoteImportError::InvalidState {
            session_id: self.record.session_id,
            state: self.record.state,
            expected: "Preparing",
        })?;
        let published = match capture.finish(&self.baseline) {
            Ok(published) => published,
            Err(error) => {
                return Err(self.persist_failure(
                    RemoteImportFailurePhase::Publish,
                    classify_failure(&error),
                    error,
                ));
            }
        };
        debug_assert!(published.session_path.is_dir());
        if let Err(error) = before_ready(&published.session_path) {
            return Err(self.persist_failure(
                RemoteImportFailurePhase::Verify,
                classify_failure(&error),
                error,
            ));
        }
        let mut prospective = self.record.clone();
        prospective.state = RemoteImportState::Ready;
        prospective.source_snapshot = Some(published.source_snapshot.clone());
        prospective.candidate = Some(published.candidate.record.clone());
        if let Err(error) = verify_published_session(&published.root, &prospective) {
            return Err(self.persist_failure(
                RemoteImportFailurePhase::Verify,
                RemoteImportFailureKind::DigestMismatch,
                error,
            ));
        }
        let result = self.store.complete_ready(
            self.record.session_id,
            self.record.generation,
            published.source_snapshot,
            published.candidate.record,
        );
        match result {
            Ok(ready) => {
                self.settled = true;
                Ok(ready)
            }
            Err(primary) => {
                if let Ok(observed) = self.store.read_session(self.record.session_id)
                    && observed.state == RemoteImportState::Ready
                    && observed.source_snapshot == prospective.source_snapshot
                    && observed.candidate == prospective.candidate
                {
                    self.settled = true;
                    return Ok(observed);
                }
                let error = self.persist_failure(
                    RemoteImportFailurePhase::Publish,
                    RemoteImportFailureKind::InvalidState,
                    primary,
                );
                Err(error)
            }
        }
    }

    fn persist_failure(
        &mut self,
        phase: RemoteImportFailurePhase,
        kind: RemoteImportFailureKind,
        primary: RemoteImportError,
    ) -> RemoteImportError {
        let failure = RemoteImportFailure { phase, kind };
        self.pending_failure = Some(failure.clone());
        let (error, settled) = fail_with_primary(&self.store, &self.record, failure, primary);
        self.settled = settled;
        error
    }

    pub(crate) fn abort(mut self) -> RemoteImportResult<RemoteImportSessionRecord> {
        let failure = self.pending_failure.clone().unwrap_or(RemoteImportFailure {
            phase: RemoteImportFailurePhase::Capture,
            kind: RemoteImportFailureKind::Interrupted,
        });
        let failed = self.store.fail(&self.record, failure)?;
        self.settled = true;
        Ok(failed)
    }
}

impl Drop for RemoteImportCapture {
    fn drop(&mut self) {
        if self.settled {
            return;
        }
        let failure = self.pending_failure.clone().unwrap_or(RemoteImportFailure {
            phase: RemoteImportFailurePhase::Capture,
            kind: RemoteImportFailureKind::Interrupted,
        });
        if self.store.fail(&self.record, failure).is_ok() {
            self.settled = true;
        }
    }
}

fn fail_with_primary(
    store: &RemoteImportStore,
    record: &RemoteImportSessionRecord,
    failure: RemoteImportFailure,
    primary: RemoteImportError,
) -> (RemoteImportError, bool) {
    match store.fail(record, failure) {
        Ok(_) => (primary, true),
        Err(state_error) => {
            let retryable = matches!(
                state_error,
                RemoteImportError::Storage(_) | RemoteImportError::Codec(_)
            );
            (
                RemoteImportError::Storage(format!(
                    "primary failure: {primary}; failed to persist session failure: {state_error}"
                )),
                !retryable,
            )
        }
    }
}

fn classify_failure(error: &RemoteImportError) -> RemoteImportFailureKind {
    match error {
        RemoteImportError::InvalidPath { .. } | RemoteImportError::DuplicatePath(_) => {
            RemoteImportFailureKind::InvalidPath
        }
        RemoteImportError::LimitExceeded { .. } => RemoteImportFailureKind::LimitExceeded,
        RemoteImportError::ArtifactTampered(_) => RemoteImportFailureKind::DigestMismatch,
        RemoteImportError::InvalidState { .. } | RemoteImportError::StaleGeneration(_) => {
            RemoteImportFailureKind::InvalidState
        }
        RemoteImportError::CandidateRevisionConflict { .. } => {
            RemoteImportFailureKind::InvalidState
        }
        RemoteImportError::SourceRead(_) => RemoteImportFailureKind::SourceRead,
        _ => RemoteImportFailureKind::ArtifactIo,
    }
}

#[allow(dead_code)]
fn _assert_revision_type_is_owned(_: RemoteImportCandidateRevision) {}
