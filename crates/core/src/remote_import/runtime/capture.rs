//! plan_ref:
//!   - 06_backup#remote-import-runtime-boundary
//!   - 06_backup#remote-import-state-machine

use super::super::artifact::{ArtifactCapture, verify_published_session};
use super::super::error::{RemoteImportError, RemoteImportResult};
use super::super::store::RemoteImportStore;
use super::super::types::{
    RemoteImportBaseline, RemoteImportFailure, RemoteImportFailureKind, RemoteImportFailurePhase,
    RemoteImportSessionId, RemoteImportSessionRecord, RemoteImportState,
};
use std::io::Read;
use std::path::Path;

pub(crate) struct RemoteImportCapture {
    pub(super) store: RemoteImportStore,
    pub(super) record: RemoteImportSessionRecord,
    pub(super) baseline: RemoteImportBaseline,
    pub(super) capture: Option<ArtifactCapture>,
    pub(super) pending_failure: Option<RemoteImportFailure>,
    pub(super) settled: bool,
}

impl RemoteImportCapture {
    pub(crate) fn session_id(&self) -> RemoteImportSessionId {
        self.record.session_id
    }

    pub(crate) fn capture_file(&mut self, path: &str, reader: impl Read) -> RemoteImportResult<()> {
        if self.pending_failure.is_some() {
            return Err(RemoteImportError::InvalidState {
                session_id: self.record.session_id,
                state: RemoteImportState::Failed,
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
                state: RemoteImportState::Failed,
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

pub(super) fn fail_with_primary(
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

pub(super) fn classify_failure(error: &RemoteImportError) -> RemoteImportFailureKind {
    match error {
        RemoteImportError::InvalidPath { .. } | RemoteImportError::DuplicatePath(_) => {
            RemoteImportFailureKind::InvalidPath
        }
        RemoteImportError::LimitExceeded { .. } => RemoteImportFailureKind::LimitExceeded,
        RemoteImportError::ArtifactTampered(_) => RemoteImportFailureKind::DigestMismatch,
        RemoteImportError::InvalidState { .. }
        | RemoteImportError::StaleGeneration(_)
        | RemoteImportError::CandidateRevisionConflict { .. } => {
            RemoteImportFailureKind::InvalidState
        }
        RemoteImportError::SourceRead(_) => RemoteImportFailureKind::SourceRead,
        _ => RemoteImportFailureKind::ArtifactIo,
    }
}
