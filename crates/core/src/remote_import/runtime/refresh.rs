//! plan_ref:
//!   - 06_backup#remote-import-runtime-boundary
//!   - 06_backup#remote-import-state-machine

use super::super::artifact::{publish_candidate_revision, verify_published_session};
use super::super::error::{RemoteImportError, RemoteImportResult};
use super::super::manifest::encode_candidate;
use super::super::types::{
    RemoteImportFailure, RemoteImportFailureKind, RemoteImportFailurePhase,
    RemoteImportRefreshRequest, RemoteImportSessionId, RemoteImportSessionRecord,
    RemoteImportState,
};
use super::RemoteImportRuntime;
use super::capture::{classify_failure, fail_with_primary};

impl RemoteImportRuntime {
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
}
