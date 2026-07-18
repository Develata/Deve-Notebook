//! plan_ref:
//!   - 03_storage/authority#remote-import-workflow-tables
//!   - 06_backup#remote-import-state-machine

use super::{RemoteImportStore, decode_session, encode};
use crate::ledger::schema::REMOTE_IMPORT_SESSIONS;
use crate::remote_import::error::{RemoteImportError, RemoteImportResult};
use crate::remote_import::types::{
    RemoteImportFailure, RemoteImportFailureKind, RemoteImportFailurePhase, RemoteImportState,
};
use redb::ReadableTable;

impl RemoteImportStore {
    pub(super) fn recover_interrupted(
        &self,
        session_id: crate::remote_import::types::RemoteImportSessionId,
        generation: u64,
    ) -> RemoteImportResult<()> {
        let write = self
            .db()
            .begin_write()
            .map_err(RemoteImportError::storage)?;
        {
            let mut sessions = write
                .open_table(REMOTE_IMPORT_SESSIONS)
                .map_err(RemoteImportError::storage)?;
            let guard = sessions
                .get(&session_id.as_u128())
                .map_err(RemoteImportError::storage)?
                .ok_or(RemoteImportError::SessionNotFound(session_id))?;
            let mut record = decode_session(session_id.as_u128(), guard.value(), self.repo_id())?;
            drop(guard);
            if record.generation != generation || record.state != RemoteImportState::Preparing {
                return Err(RemoteImportError::Storage(
                    "Remote Import recovery target changed after preflight".to_string(),
                ));
            }
            record.state = RemoteImportState::Failed;
            record.failure = Some(RemoteImportFailure {
                phase: RemoteImportFailurePhase::Recovery,
                kind: RemoteImportFailureKind::Interrupted,
            });
            let bytes = encode(&record)?;
            sessions
                .insert(&record.session_id.as_u128(), bytes.as_slice())
                .map_err(RemoteImportError::storage)?;
        }
        write.commit().map_err(RemoteImportError::storage)?;
        Ok(())
    }
}
