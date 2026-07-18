//! plan_ref:
//!   - 03_storage/authority#remote-import-workflow-tables
//!   - 06_backup#remote-import-state-machine

use super::{
    RUNTIME_KEY, RemoteImportStore, decode_runtime, decode_session, encode,
    retention::prune_terminal_records,
};
use crate::ledger::schema::{REMOTE_IMPORT_RUNTIME, REMOTE_IMPORT_SESSIONS};
use crate::models::GlobalSeq;
use crate::remote_import::error::{RemoteImportError, RemoteImportResult};
use crate::remote_import::types::{
    REMOTE_IMPORT_VALUE_VERSION, RemoteImportBranch, RemoteImportCandidateRevision,
    RemoteImportCandidateRevisionRecord, RemoteImportFailure, RemoteImportSessionId,
    RemoteImportSessionRecord, RemoteImportSourceSnapshot, RemoteImportState,
};
use redb::ReadableTable;

impl RemoteImportStore {
    pub(in crate::remote_import) fn reserve(
        &self,
        source_binding_digest: crate::remote_import::types::RemoteImportDigest,
        locator_binding_digest: crate::remote_import::types::RemoteImportDigest,
        baseline_head: GlobalSeq,
        ignore_digest: crate::remote_import::types::RemoteImportDigest,
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        let write = self
            .db()
            .begin_write()
            .map_err(RemoteImportError::storage)?;
        let record;
        {
            let mut runtime_table = write
                .open_table(REMOTE_IMPORT_RUNTIME)
                .map_err(RemoteImportError::storage)?;
            let runtime_guard = runtime_table
                .get(&RUNTIME_KEY)
                .map_err(RemoteImportError::storage)?
                .ok_or_else(|| RemoteImportError::Storage("runtime row missing".to_string()))?;
            let mut runtime = decode_runtime(runtime_guard.value())?;
            drop(runtime_guard);
            if let Some(active) = runtime.active_session {
                return Err(RemoteImportError::ActiveSession(active));
            }
            let session_id = RemoteImportSessionId::new();
            let generation = runtime.next_generation;
            let order = runtime.next_order;
            runtime.next_generation = runtime.next_generation.checked_add(1).ok_or_else(|| {
                RemoteImportError::Storage("Remote Import generation overflow".to_string())
            })?;
            runtime.next_order = runtime.next_order.checked_add(1).ok_or_else(|| {
                RemoteImportError::Storage("Remote Import order overflow".to_string())
            })?;
            runtime.active_session = Some(session_id);
            record = RemoteImportSessionRecord {
                value_version: REMOTE_IMPORT_VALUE_VERSION,
                session_id,
                repo_id: self.repo_id(),
                branch: RemoteImportBranch::Local,
                generation,
                order,
                state: RemoteImportState::Preparing,
                source_binding_digest,
                locator_binding_digest,
                baseline_head,
                ignore_digest,
                source_snapshot: None,
                candidate: None,
                failure: None,
                apply_receipt: None,
                cleanup_pending: false,
            };
            let mut sessions = write
                .open_table(REMOTE_IMPORT_SESSIONS)
                .map_err(RemoteImportError::storage)?;
            let record_bytes = encode(&record)?;
            sessions
                .insert(&session_id.as_u128(), record_bytes.as_slice())
                .map_err(RemoteImportError::storage)?;
            drop(sessions);
            let runtime_bytes = encode(&runtime)?;
            runtime_table
                .insert(&RUNTIME_KEY, runtime_bytes.as_slice())
                .map_err(RemoteImportError::storage)?;
        }
        write.commit().map_err(RemoteImportError::storage)?;
        Ok(record)
    }

    pub(in crate::remote_import) fn complete_ready(
        &self,
        session_id: RemoteImportSessionId,
        generation: u64,
        source_snapshot: RemoteImportSourceSnapshot,
        candidate: RemoteImportCandidateRevisionRecord,
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        self.update_exact(session_id, generation, "Preparing", |record| {
            if record.state != RemoteImportState::Preparing {
                return Err(invalid_state(record, "Preparing"));
            }
            if candidate.ledger_head != record.baseline_head
                || candidate.ignore_digest != record.ignore_digest
            {
                return Err(RemoteImportError::ArtifactTampered(
                    "prepared candidate does not match reserved session baseline".to_string(),
                ));
            }
            record.state = RemoteImportState::Ready;
            record.source_snapshot = Some(source_snapshot);
            record.candidate = Some(candidate);
            record.failure = None;
            Ok(())
        })
    }

    pub(in crate::remote_import) fn fail(
        &self,
        expected: &RemoteImportSessionRecord,
        failure: RemoteImportFailure,
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        self.update_exact(
            expected.session_id,
            expected.generation,
            "active",
            |record| {
                if record.state == RemoteImportState::Failed {
                    if record.failure.as_ref() == Some(&failure) {
                        return Ok(());
                    }
                    return Err(invalid_state(record, "same persisted failure"));
                }
                if record != expected {
                    return Err(RemoteImportError::StaleGeneration(expected.session_id));
                }
                if record.state.is_terminal() {
                    return Err(invalid_state(record, "active"));
                }
                record.state = RemoteImportState::Failed;
                record.failure = Some(failure);
                Ok(())
            },
        )
    }

    pub(in crate::remote_import) fn update_candidate(
        &self,
        session_id: RemoteImportSessionId,
        generation: u64,
        expected: &RemoteImportCandidateRevisionRecord,
        candidate: RemoteImportCandidateRevisionRecord,
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        self.update_exact(session_id, generation, "Ready or Stale", |record| {
            if !matches!(
                record.state,
                RemoteImportState::Ready | RemoteImportState::Stale
            ) {
                return Err(invalid_state(record, "Ready or Stale"));
            }
            if record.candidate.as_ref() != Some(expected) {
                return Err(RemoteImportError::StaleGeneration(session_id));
            }
            record.baseline_head = candidate.ledger_head;
            record.ignore_digest = candidate.ignore_digest;
            record.candidate = Some(candidate);
            record.state = RemoteImportState::Ready;
            record.failure = None;
            Ok(())
        })
    }

    pub(in crate::remote_import) fn mark_stale(
        &self,
        session_id: RemoteImportSessionId,
        generation: u64,
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        self.update_exact(session_id, generation, "Ready or Stale", |record| {
            if !matches!(
                record.state,
                RemoteImportState::Ready | RemoteImportState::Stale
            ) {
                return Err(invalid_state(record, "Ready or Stale"));
            }
            record.state = RemoteImportState::Stale;
            record.failure = None;
            Ok(())
        })
    }

    pub(in crate::remote_import) fn begin_discard(
        &self,
        session_id: RemoteImportSessionId,
        generation: u64,
        expected_revision: Option<RemoteImportCandidateRevision>,
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        let write = self
            .db()
            .begin_write()
            .map_err(RemoteImportError::storage)?;
        let record;
        {
            let mut sessions = write
                .open_table(REMOTE_IMPORT_SESSIONS)
                .map_err(RemoteImportError::storage)?;
            let guard = sessions
                .get(&session_id.as_u128())
                .map_err(RemoteImportError::storage)?
                .ok_or(RemoteImportError::SessionNotFound(session_id))?;
            let mut current = decode_session(session_id.as_u128(), guard.value(), self.repo_id())?;
            drop(guard);
            if current.generation != generation {
                return Err(RemoteImportError::StaleGeneration(session_id));
            }
            let observed_revision = current
                .candidate
                .as_ref()
                .map(|candidate| candidate.revision);
            if observed_revision != expected_revision {
                return Err(RemoteImportError::Stale {
                    session_id,
                    blockers: Vec::new(),
                });
            }
            if !matches!(
                current.state,
                RemoteImportState::Ready | RemoteImportState::Stale | RemoteImportState::Failed
            ) {
                return Err(invalid_state(&current, "Ready, Stale, or Failed"));
            }
            current.state = RemoteImportState::Discarded;
            current.cleanup_pending = true;
            current.failure = None;
            let bytes = encode(&current)?;
            sessions
                .insert(&session_id.as_u128(), bytes.as_slice())
                .map_err(RemoteImportError::storage)?;
            drop(sessions);
            let mut runtime_table = write
                .open_table(REMOTE_IMPORT_RUNTIME)
                .map_err(RemoteImportError::storage)?;
            let runtime_guard = runtime_table
                .get(&RUNTIME_KEY)
                .map_err(RemoteImportError::storage)?
                .ok_or_else(|| RemoteImportError::Storage("runtime row missing".to_string()))?;
            let mut runtime = decode_runtime(runtime_guard.value())?;
            drop(runtime_guard);
            if runtime.active_session != Some(session_id) {
                return Err(RemoteImportError::StaleGeneration(session_id));
            }
            runtime.active_session = None;
            let bytes = encode(&runtime)?;
            runtime_table
                .insert(&RUNTIME_KEY, bytes.as_slice())
                .map_err(RemoteImportError::storage)?;
            record = current;
        }
        write.commit().map_err(RemoteImportError::storage)?;
        Ok(record)
    }

    pub(in crate::remote_import) fn finish_cleanup(
        &self,
        session_id: RemoteImportSessionId,
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        let write = self
            .db()
            .begin_write()
            .map_err(RemoteImportError::storage)?;
        let record;
        {
            let mut sessions = write
                .open_table(REMOTE_IMPORT_SESSIONS)
                .map_err(RemoteImportError::storage)?;
            let guard = sessions
                .get(&session_id.as_u128())
                .map_err(RemoteImportError::storage)?
                .ok_or(RemoteImportError::SessionNotFound(session_id))?;
            let mut current = decode_session(session_id.as_u128(), guard.value(), self.repo_id())?;
            drop(guard);
            if !current.state.is_terminal() || !current.cleanup_pending {
                return Err(invalid_state(&current, "terminal with cleanup_pending"));
            }
            current.cleanup_pending = false;
            let bytes = encode(&current)?;
            sessions
                .insert(&session_id.as_u128(), bytes.as_slice())
                .map_err(RemoteImportError::storage)?;
            prune_terminal_records(&mut sessions, self.repo_id())?;
            record = current;
        }
        write.commit().map_err(RemoteImportError::storage)?;
        Ok(record)
    }

    fn update_exact(
        &self,
        session_id: RemoteImportSessionId,
        generation: u64,
        _expected: &'static str,
        update: impl FnOnce(&mut RemoteImportSessionRecord) -> RemoteImportResult<()>,
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        let write = self
            .db()
            .begin_write()
            .map_err(RemoteImportError::storage)?;
        let record;
        {
            let mut sessions = write
                .open_table(REMOTE_IMPORT_SESSIONS)
                .map_err(RemoteImportError::storage)?;
            let guard = sessions
                .get(&session_id.as_u128())
                .map_err(RemoteImportError::storage)?
                .ok_or(RemoteImportError::SessionNotFound(session_id))?;
            let mut current = decode_session(session_id.as_u128(), guard.value(), self.repo_id())?;
            drop(guard);
            if current.generation != generation {
                return Err(RemoteImportError::StaleGeneration(session_id));
            }
            update(&mut current)?;
            let bytes = encode(&current)?;
            sessions
                .insert(&session_id.as_u128(), bytes.as_slice())
                .map_err(RemoteImportError::storage)?;
            record = current;
        }
        write.commit().map_err(RemoteImportError::storage)?;
        Ok(record)
    }
}

fn invalid_state(record: &RemoteImportSessionRecord, expected: &'static str) -> RemoteImportError {
    RemoteImportError::InvalidState {
        session_id: record.session_id,
        state: record.state,
        expected,
    }
}
