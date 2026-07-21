//! plan_ref:
//!   - 03_storage/authority#remote-import-workflow-tables
//!   - 06_backup#remote-import-state-machine

mod database;
mod recovery;
pub(super) mod retention;
mod transitions;

use crate::ledger::RepoManager;
use crate::ledger::manager::BoundRepoAuthority;
use crate::ledger::schema::{REMOTE_IMPORT_RUNTIME, REMOTE_IMPORT_SESSIONS};
use crate::models::RepoId;
use crate::remote_import::error::{RemoteImportError, RemoteImportResult};
use crate::remote_import::types::{
    REMOTE_IMPORT_VALUE_VERSION, RemoteImportRuntimeRecord, RemoteImportSessionId,
    RemoteImportSessionRecord, RemoteImportState,
};
use redb::{Database, ReadableTable};
use std::collections::BTreeSet;
use std::sync::Arc;

#[cfg(test)]
pub(super) use self::database::RemoteImportTestDatabase;
use self::database::{StoreDatabase, StoreDatabaseLease};

pub(super) const RUNTIME_KEY: u8 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StoreOpenState {
    Empty,
    Ready,
    RecoverPreparing {
        session_id: RemoteImportSessionId,
        generation: u64,
    },
}

#[derive(Clone, Copy)]
enum StoreOpenMode {
    ActiveProcess,
    RecoverStartup,
}

#[derive(Clone)]
pub(super) struct RemoteImportStore {
    db: Arc<StoreDatabase>,
    repo_id: RepoId,
}

impl RemoteImportStore {
    pub(super) fn validate_schema(db: &Database) -> RemoteImportResult<()> {
        RepoManager::validate_local_repo_schema(db).map_err(RemoteImportError::storage)
    }

    #[cfg(test)]
    pub(super) fn open(
        db: impl Into<RemoteImportTestDatabase>,
        repo_id: RepoId,
    ) -> RemoteImportResult<Self> {
        Self::open_with_mode(db.into().0, repo_id, StoreOpenMode::ActiveProcess)
    }

    pub(super) fn open_authority(
        authority: BoundRepoAuthority,
        repo_id: RepoId,
    ) -> RemoteImportResult<Self> {
        Self::open_with_mode(
            Arc::new(StoreDatabase::Authority(authority)),
            repo_id,
            StoreOpenMode::ActiveProcess,
        )
    }

    pub(super) fn recover_startup_authority(
        authority: BoundRepoAuthority,
        repo_id: RepoId,
    ) -> RemoteImportResult<Self> {
        Self::open_with_mode(
            Arc::new(StoreDatabase::Authority(authority)),
            repo_id,
            StoreOpenMode::RecoverStartup,
        )
    }

    fn open_with_mode(
        db: Arc<StoreDatabase>,
        repo_id: RepoId,
        mode: StoreOpenMode,
    ) -> RemoteImportResult<Self> {
        let lease = db.lease()?;
        Self::validate_schema(&lease)?;
        drop(lease);
        let store = Self { db, repo_id };
        match store.preflight_open()? {
            StoreOpenState::Empty => store.initialize_empty_runtime()?,
            StoreOpenState::Ready => {}
            StoreOpenState::RecoverPreparing {
                session_id,
                generation,
            } if matches!(mode, StoreOpenMode::RecoverStartup) => {
                store.recover_interrupted(session_id, generation)?
            }
            StoreOpenState::RecoverPreparing { .. } => {}
        }
        let final_state = store.preflight_open()?;
        let valid = matches!(final_state, StoreOpenState::Ready)
            || matches!(
                (mode, final_state),
                (
                    StoreOpenMode::ActiveProcess,
                    StoreOpenState::RecoverPreparing { .. }
                )
            );
        if !valid {
            return Err(RemoteImportError::Storage(
                "Remote Import store did not reach a valid opened state".to_string(),
            ));
        }
        Ok(store)
    }

    #[cfg(test)]
    pub(super) fn open_read_only(
        db: impl Into<RemoteImportTestDatabase>,
        repo_id: RepoId,
    ) -> RemoteImportResult<Self> {
        let db = db.into().0;
        let lease = db.lease()?;
        Self::validate_schema(&lease)?;
        drop(lease);
        let store = Self { db, repo_id };
        store.preflight_open()?;
        Ok(store)
    }

    pub(super) fn open_read_only_authority(
        authority: BoundRepoAuthority,
        repo_id: RepoId,
    ) -> RemoteImportResult<Self> {
        let db = Arc::new(StoreDatabase::Authority(authority));
        let lease = db.lease()?;
        Self::validate_schema(&lease)?;
        drop(lease);
        let store = Self { db, repo_id };
        store.preflight_open()?;
        Ok(store)
    }

    pub(super) fn with_db<R>(
        &self,
        inspect: impl FnOnce(&Database) -> RemoteImportResult<R>,
    ) -> RemoteImportResult<R> {
        let lease = self.db.lease()?;
        inspect(&lease)
    }

    pub(in crate::remote_import) fn lease_db(&self) -> RemoteImportResult<StoreDatabaseLease<'_>> {
        self.db.lease()
    }

    pub(super) fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub(super) fn read_session(
        &self,
        session_id: RemoteImportSessionId,
    ) -> RemoteImportResult<RemoteImportSessionRecord> {
        let db = self.db.lease()?;
        let read = db.begin_read().map_err(RemoteImportError::storage)?;
        let sessions = read
            .open_table(REMOTE_IMPORT_SESSIONS)
            .map_err(RemoteImportError::storage)?;
        let guard = sessions
            .get(&session_id.as_u128())
            .map_err(RemoteImportError::storage)?
            .ok_or(RemoteImportError::SessionNotFound(session_id))?;
        decode_session(session_id.as_u128(), guard.value(), self.repo_id)
    }

    pub(super) fn list_sessions(&self) -> RemoteImportResult<Vec<RemoteImportSessionRecord>> {
        let db = self.db.lease()?;
        let read = db.begin_read().map_err(RemoteImportError::storage)?;
        let sessions = read
            .open_table(REMOTE_IMPORT_SESSIONS)
            .map_err(RemoteImportError::storage)?;
        let mut records = Vec::new();
        let iter = sessions.iter().map_err(RemoteImportError::storage)?;
        for row in iter {
            let (key, value) = row.map_err(RemoteImportError::storage)?;
            records.push(decode_session(key.value(), value.value(), self.repo_id)?);
        }
        records.sort_by_key(|record| record.order);
        Ok(records)
    }

    /// Reads the runtime generation and all session rows from one Redb read
    /// transaction so repo-removal admission can be revalidated exactly.
    pub(super) fn repo_removal_observation(
        &self,
    ) -> RemoteImportResult<(u64, Vec<RemoteImportSessionRecord>)> {
        let db = self.db.lease()?;
        let read = db.begin_read().map_err(RemoteImportError::storage)?;
        let sessions = read
            .open_table(REMOTE_IMPORT_SESSIONS)
            .map_err(RemoteImportError::storage)?;
        let mut records = Vec::new();
        for row in sessions.iter().map_err(RemoteImportError::storage)? {
            let (key, value) = row.map_err(RemoteImportError::storage)?;
            records.push(decode_session(key.value(), value.value(), self.repo_id)?);
        }
        records.sort_by_key(|record| record.order);
        drop(sessions);

        let runtime_table = read
            .open_table(REMOTE_IMPORT_RUNTIME)
            .map_err(RemoteImportError::storage)?;
        let runtime = runtime_table
            .get(&RUNTIME_KEY)
            .map_err(RemoteImportError::storage)?
            .ok_or_else(|| {
                RemoteImportError::Storage(
                    "Remote Import runtime row is missing during repo-removal admission"
                        .to_string(),
                )
            })?;
        let runtime = decode_runtime(runtime.value())?;
        validate_record_shapes(&records, &runtime)?;
        let active = records
            .iter()
            .filter(|record| !record.state.is_terminal())
            .map(|record| record.session_id)
            .collect::<Vec<_>>();
        match (runtime.active_session, active.as_slice()) {
            (None, []) | (Some(_), [_]) if runtime.active_session == active.first().copied() => {}
            _ => {
                return Err(RemoteImportError::Storage(
                    "Remote Import active-session invariant is corrupt during repo-removal admission"
                        .to_string(),
                ));
            }
        }
        Ok((runtime.next_generation, records))
    }

    fn initialize_empty_runtime(&self) -> RemoteImportResult<()> {
        let db = self.db.lease()?;
        let write = db.begin_write().map_err(RemoteImportError::storage)?;
        {
            let sessions = write
                .open_table(REMOTE_IMPORT_SESSIONS)
                .map_err(RemoteImportError::storage)?;
            if sessions
                .iter()
                .map_err(RemoteImportError::storage)?
                .next()
                .transpose()
                .map_err(RemoteImportError::storage)?
                .is_some()
            {
                return Err(RemoteImportError::Storage(
                    "refusing to initialize Remote Import runtime with existing sessions"
                        .to_string(),
                ));
            }
            drop(sessions);
            let mut runtime = write
                .open_table(REMOTE_IMPORT_RUNTIME)
                .map_err(RemoteImportError::storage)?;
            if runtime
                .iter()
                .map_err(RemoteImportError::storage)?
                .next()
                .transpose()
                .map_err(RemoteImportError::storage)?
                .is_some()
            {
                return Err(RemoteImportError::Storage(
                    "refusing to initialize non-empty Remote Import runtime table".to_string(),
                ));
            }
            let bytes = encode(&RemoteImportRuntimeRecord::default())?;
            runtime
                .insert(&RUNTIME_KEY, bytes.as_slice())
                .map_err(RemoteImportError::storage)?;
        }
        write.commit().map_err(RemoteImportError::storage)
    }

    fn preflight_open(&self) -> RemoteImportResult<StoreOpenState> {
        let db = self.db.lease()?;
        let read = db.begin_read().map_err(RemoteImportError::storage)?;
        let sessions = read
            .open_table(REMOTE_IMPORT_SESSIONS)
            .map_err(RemoteImportError::storage)?;
        let mut records = Vec::new();
        for row in sessions.iter().map_err(RemoteImportError::storage)? {
            let (key, value) = row.map_err(RemoteImportError::storage)?;
            records.push(decode_session(key.value(), value.value(), self.repo_id)?);
        }
        drop(sessions);
        let runtime_table = read
            .open_table(REMOTE_IMPORT_RUNTIME)
            .map_err(RemoteImportError::storage)?;
        let mut runtime_rows = runtime_table.iter().map_err(RemoteImportError::storage)?;
        let Some(runtime_row) = runtime_rows
            .next()
            .transpose()
            .map_err(RemoteImportError::storage)?
        else {
            return if records.is_empty() {
                Ok(StoreOpenState::Empty)
            } else {
                Err(RemoteImportError::Storage(
                    "Remote Import runtime row is missing while sessions exist".to_string(),
                ))
            };
        };
        let (runtime_key, runtime_value) = runtime_row;
        if runtime_key.value() != RUNTIME_KEY
            || runtime_rows
                .next()
                .transpose()
                .map_err(RemoteImportError::storage)?
                .is_some()
        {
            return Err(RemoteImportError::Storage(
                "Remote Import runtime table contains unexpected rows".to_string(),
            ));
        }
        let runtime = decode_runtime(runtime_value.value())?;
        validate_record_shapes(&records, &runtime)?;
        let active_records = records
            .iter()
            .filter(|record| !record.state.is_terminal())
            .collect::<Vec<_>>();
        match (runtime.active_session, active_records.as_slice()) {
            (None, []) => Ok(StoreOpenState::Ready),
            (Some(active), [record]) if active == record.session_id => {
                if record.state == RemoteImportState::Preparing {
                    Ok(StoreOpenState::RecoverPreparing {
                        session_id: record.session_id,
                        generation: record.generation,
                    })
                } else {
                    Ok(StoreOpenState::Ready)
                }
            }
            _ => Err(RemoteImportError::Storage(
                "Remote Import active-session invariant is corrupt".to_string(),
            )),
        }
    }
}

fn validate_record_shapes(
    records: &[RemoteImportSessionRecord],
    runtime: &RemoteImportRuntimeRecord,
) -> RemoteImportResult<()> {
    if runtime.next_generation == 0 || runtime.next_order == 0 {
        return Err(RemoteImportError::Storage(
            "Remote Import next generation/order invariant is corrupt".to_string(),
        ));
    }
    let mut generations = BTreeSet::new();
    let mut orders = BTreeSet::new();
    for record in records {
        if record.generation == 0
            || record.order == 0
            || !generations.insert(record.generation)
            || !orders.insert(record.order)
            || record.generation >= runtime.next_generation
            || record.order >= runtime.next_order
        {
            return Err(RemoteImportError::Storage(
                "Remote Import generation/order invariant is corrupt".to_string(),
            ));
        }
        if record
            .candidate
            .as_ref()
            .is_some_and(|candidate| candidate.revision.get() == 0)
            || record
                .apply_receipt
                .as_ref()
                .is_some_and(|receipt| receipt.revision.get() == 0)
        {
            return Err(RemoteImportError::Storage(format!(
                "Remote Import session {} candidate revision invariant is corrupt",
                record.session_id
            )));
        }
        let valid_shape = match record.state {
            RemoteImportState::Preparing => {
                record.source_snapshot.is_none()
                    && record.candidate.is_none()
                    && record.failure.is_none()
                    && record.apply_receipt.is_none()
                    && !record.cleanup_pending
            }
            RemoteImportState::Ready | RemoteImportState::Stale => {
                record.source_snapshot.is_some()
                    && record.candidate.is_some()
                    && record.failure.is_none()
                    && record.apply_receipt.is_none()
                    && !record.cleanup_pending
            }
            RemoteImportState::Failed => {
                record.failure.is_some()
                    && record.apply_receipt.is_none()
                    && !record.cleanup_pending
            }
            RemoteImportState::Applied => record.apply_receipt.is_some(),
            RemoteImportState::Discarded => record.apply_receipt.is_none(),
        };
        if !valid_shape {
            return Err(RemoteImportError::Storage(format!(
                "Remote Import session {} state payload is corrupt",
                record.session_id
            )));
        }
    }
    Ok(())
}

pub(super) fn encode<T: serde::Serialize + ?Sized>(value: &T) -> RemoteImportResult<Vec<u8>> {
    crate::codec::encode(value).map_err(RemoteImportError::codec)
}

pub(super) fn decode_runtime(bytes: &[u8]) -> RemoteImportResult<RemoteImportRuntimeRecord> {
    let value: RemoteImportRuntimeRecord =
        crate::codec::decode(bytes).map_err(RemoteImportError::codec)?;
    if value.value_version != REMOTE_IMPORT_VALUE_VERSION {
        return Err(RemoteImportError::Codec(format!(
            "unsupported Remote Import runtime value version {}",
            value.value_version
        )));
    }
    Ok(value)
}

pub(super) fn decode_session(
    key: u128,
    bytes: &[u8],
    repo_id: RepoId,
) -> RemoteImportResult<RemoteImportSessionRecord> {
    let value: RemoteImportSessionRecord =
        crate::codec::decode(bytes).map_err(RemoteImportError::codec)?;
    if !value.validate_value_version() {
        return Err(RemoteImportError::Codec(format!(
            "unsupported Remote Import session value version {}",
            value.value_version
        )));
    }
    if value.session_id.as_u128() != key || value.repo_id != repo_id {
        return Err(RemoteImportError::Storage(
            "Remote Import session identity does not match table/repo identity".to_string(),
        ));
    }
    Ok(value)
}
