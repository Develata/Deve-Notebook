//! plan_ref:
//!   - 03_storage/projection#durable-projection-fault-contract
//!   - 03_storage/authority#redb-schema-version-contract
//!   - 03_storage/authority#projection-fault-recovery-table

use super::types::{
    DurableProjectionFault, MAX_ERROR_CHARS, PROJECTION_FAULT_VALUE_VERSION,
    PreparedProjectionFault, ProjectionFaultError, ProjectionFaultInput, ProjectionFaultKind,
    ProjectionFaultOrigin, ProjectionFaultResult, ProjectionFaultStatus,
};
use crate::ledger::schema::{PROJECTION_FAULTS, REPO_INFO_METADATA_KEY, REPO_METADATA};
use crate::ledger::{RepoInfo, RepoManager};
use crate::models::{DocId, RepoId};
use crate::utils::path::{path_to_forward_slash, to_forward_slash};
use redb::ReadableTable;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const KEY_DOMAIN: &[u8] = b"deve-projection-fault-key-v1\0";

pub(crate) fn record_fault(
    repo: &RepoManager,
    repo_name: &str,
    input: ProjectionFaultInput<'_>,
) -> ProjectionFaultResult<()> {
    let info = repo
        .get_repo_info_for(None, Some(repo_name))
        .map_err(ProjectionFaultError::storage)?
        .ok_or_else(|| {
            ProjectionFaultError::Invariant(format!(
                "local repo metadata is missing for {repo_name}"
            ))
        })?;
    let execution_name = repo
        .resolve_local_repo_name_for_execution(Some(info.uuid), Some(repo_name))
        .map_err(ProjectionFaultError::storage)?;
    let workspace_root = repo
        .local_repo_workspace_root(&execution_name)
        .ok()
        .map(|path| path_to_forward_slash(&path));
    let origin = match input.fault_kind {
        ProjectionFaultKind::ProjectionWritebackFailed => {
            ProjectionFaultOrigin::ProjectionPersistence
        }
        ProjectionFaultKind::ProjectionRebuildInterrupted => {
            ProjectionFaultOrigin::ProjectionRepair
        }
    };
    let prepared = prepare(
        &info,
        origin,
        input.fault_kind,
        input.target_path,
        input.source_path,
        input.doc_id,
        input.ledger_seq_or_head,
        workspace_root,
        input.last_error,
    );
    repo.run_on_local_repo(&execution_name, |db| {
        let write = db.begin_write().map_err(ProjectionFaultError::storage)?;
        record_prepared_in_txn(&write, info.uuid, &prepared)?;
        write.commit().map_err(ProjectionFaultError::storage)?;
        Ok(())
    })
    .map_err(ProjectionFaultError::storage)
}

pub(crate) fn prepare_remote_import_fault(
    repo_id: RepoId,
    repo_name_at_fault: String,
    session_id: u128,
    revision: u64,
    request_id: Uuid,
    ledger_head: u64,
    last_error: &str,
) -> PreparedProjectionFault {
    let info = RepoInfo {
        uuid: repo_id,
        name: repo_name_at_fault,
        url: None,
    };
    prepare(
        &info,
        ProjectionFaultOrigin::RemoteImport {
            session_id,
            revision,
            request_id,
        },
        ProjectionFaultKind::ProjectionWritebackFailed,
        None,
        None,
        None,
        Some(ledger_head),
        None,
        last_error,
    )
}

#[allow(clippy::too_many_arguments)]
fn prepare(
    info: &RepoInfo,
    origin: ProjectionFaultOrigin,
    fault_kind: ProjectionFaultKind,
    target_path: Option<&str>,
    source_path: Option<&str>,
    doc_id: Option<DocId>,
    ledger_seq_or_head: Option<u64>,
    projection_workspace_root: Option<String>,
    last_error: &str,
) -> PreparedProjectionFault {
    let target_path = target_path.map(to_forward_slash);
    let source_path = source_path.map(to_forward_slash);
    let now = chrono::Utc::now().timestamp_millis();
    let value = DurableProjectionFault {
        value_version: PROJECTION_FAULT_VALUE_VERSION,
        repo_id: info.uuid,
        repo_name_at_fault: info.name.clone(),
        name_epoch: None,
        fault_kind,
        origin,
        target_path,
        source_path,
        doc_id,
        ledger_seq_or_head,
        projection_workspace_root,
        first_seen_at_unix_ms: now,
        last_seen_at_unix_ms: now,
        last_error: bounded_error(last_error),
        retry_count: 1,
        status: ProjectionFaultStatus::Pending,
    };
    PreparedProjectionFault {
        key: key_for(&value),
        value,
    }
}

pub(crate) fn record_prepared_in_txn(
    write: &redb::WriteTransaction,
    expected_repo_id: RepoId,
    prepared: &PreparedProjectionFault,
) -> ProjectionFaultResult<()> {
    verify_repo_identity_in_txn(write, expected_repo_id)?;
    if prepared.value.repo_id != expected_repo_id || key_for(&prepared.value) != prepared.key {
        return Err(ProjectionFaultError::Invariant(
            "prepared Projection Fault identity does not match target repo/key".to_string(),
        ));
    }
    let mut table = write
        .open_table(PROJECTION_FAULTS)
        .map_err(ProjectionFaultError::storage)?;
    let value = match table
        .get(&prepared.key)
        .map_err(ProjectionFaultError::storage)?
    {
        Some(existing) => {
            let mut current = decode_fault(prepared.key, existing.value(), expected_repo_id)?;
            if !same_identity(&current, &prepared.value) {
                return Err(ProjectionFaultError::Invariant(
                    "Projection Fault key collision or identity drift".to_string(),
                ));
            }
            current.repo_name_at_fault = prepared.value.repo_name_at_fault.clone();
            current.name_epoch = prepared.value.name_epoch;
            current.ledger_seq_or_head = prepared.value.ledger_seq_or_head;
            current.projection_workspace_root = prepared.value.projection_workspace_root.clone();
            current.last_seen_at_unix_ms = prepared.value.last_seen_at_unix_ms;
            current.last_error = prepared.value.last_error.clone();
            current.retry_count = current.retry_count.saturating_add(1);
            current
        }
        None => prepared.value.clone(),
    };
    let bytes = crate::codec::encode(&value).map_err(ProjectionFaultError::codec)?;
    table
        .insert(&prepared.key, bytes.as_slice())
        .map_err(ProjectionFaultError::storage)?;
    Ok(())
}

pub(crate) fn load_degraded_repo_ids(repo: &RepoManager) -> ProjectionFaultResult<Vec<RepoId>> {
    let mut degraded = Vec::new();
    let names = repo
        .list_local_repo_names_for_execution()
        .map_err(ProjectionFaultError::storage)?;
    for execution_name in names {
        let repo_id = Uuid::parse_str(&execution_name).map_err(|_| {
            ProjectionFaultError::Invariant(format!(
                "local repo execution name is not a RepoId: {execution_name}"
            ))
        })?;
        let has_fault = repo
            .run_on_local_repo(&execution_name, |db| {
                let read = db.begin_read().map_err(ProjectionFaultError::storage)?;
                let table = read
                    .open_table(PROJECTION_FAULTS)
                    .map_err(ProjectionFaultError::storage)?;
                for row in table.iter().map_err(ProjectionFaultError::storage)? {
                    let (key, value) = row.map_err(ProjectionFaultError::storage)?;
                    let fault = decode_fault(key.value(), value.value(), repo_id)?;
                    if fault.status == ProjectionFaultStatus::Pending {
                        return Ok(true);
                    }
                }
                Ok(false)
            })
            .map_err(ProjectionFaultError::storage)?;
        if has_fault {
            degraded.push(repo_id);
        }
    }
    degraded.sort();
    degraded.dedup();
    Ok(degraded)
}

#[cfg(test)]
pub(crate) fn remote_import_origins_for_test(
    db: &redb::Database,
    expected_repo_id: RepoId,
) -> ProjectionFaultResult<Vec<(u128, u64, Uuid)>> {
    let read = db.begin_read().map_err(ProjectionFaultError::storage)?;
    let table = read
        .open_table(PROJECTION_FAULTS)
        .map_err(ProjectionFaultError::storage)?;
    let mut origins = Vec::new();
    for row in table.iter().map_err(ProjectionFaultError::storage)? {
        let (key, value) = row.map_err(ProjectionFaultError::storage)?;
        let fault = decode_fault(key.value(), value.value(), expected_repo_id)?;
        if let ProjectionFaultOrigin::RemoteImport {
            session_id,
            revision,
            request_id,
        } = fault.origin
        {
            origins.push((session_id, revision, request_id));
        }
    }
    origins.sort();
    Ok(origins)
}

pub(crate) fn clear_faults_for_repo(
    repo: &RepoManager,
    repo_name: &str,
) -> ProjectionFaultResult<()> {
    let info = repo
        .get_repo_info_for(None, Some(repo_name))
        .map_err(ProjectionFaultError::storage)?
        .ok_or_else(|| {
            ProjectionFaultError::Invariant(format!(
                "local repo metadata is missing for {repo_name}"
            ))
        })?;
    let execution_name = repo
        .resolve_local_repo_name_for_execution(Some(info.uuid), Some(repo_name))
        .map_err(ProjectionFaultError::storage)?;
    repo.run_on_local_repo(&execution_name, |db| {
        let write = db.begin_write().map_err(ProjectionFaultError::storage)?;
        verify_repo_identity_in_txn(&write, info.uuid)?;
        {
            let mut table = write
                .open_table(PROJECTION_FAULTS)
                .map_err(ProjectionFaultError::storage)?;
            let keys = table
                .iter()
                .map_err(ProjectionFaultError::storage)?
                .map(|row| {
                    let (key, value) = row.map_err(ProjectionFaultError::storage)?;
                    let key = key.value();
                    decode_fault(key, value.value(), info.uuid)?;
                    Ok(key)
                })
                .collect::<ProjectionFaultResult<Vec<_>>>()?;
            for key in keys {
                table.remove(&key).map_err(ProjectionFaultError::storage)?;
            }
        }
        write.commit().map_err(ProjectionFaultError::storage)?;
        Ok(())
    })
    .map_err(ProjectionFaultError::storage)
}

fn verify_repo_identity_in_txn(
    write: &redb::WriteTransaction,
    expected_repo_id: RepoId,
) -> ProjectionFaultResult<RepoInfo> {
    let metadata = write
        .open_table(REPO_METADATA)
        .map_err(ProjectionFaultError::storage)?;
    let value = metadata
        .get(&REPO_INFO_METADATA_KEY)
        .map_err(ProjectionFaultError::storage)?
        .ok_or_else(|| ProjectionFaultError::Invariant("local RepoInfo is missing".to_string()))?;
    let info: RepoInfo =
        crate::codec::decode(value.value()).map_err(ProjectionFaultError::codec)?;
    if info.uuid != expected_repo_id {
        return Err(ProjectionFaultError::Invariant(format!(
            "Projection Fault target RepoId {} differs from local RepoId {}",
            expected_repo_id, info.uuid
        )));
    }
    Ok(info)
}

fn decode_fault(
    key: [u8; 32],
    bytes: &[u8],
    expected_repo_id: RepoId,
) -> ProjectionFaultResult<DurableProjectionFault> {
    let value: DurableProjectionFault =
        crate::codec::decode(bytes).map_err(ProjectionFaultError::codec)?;
    if value.value_version != PROJECTION_FAULT_VALUE_VERSION {
        return Err(ProjectionFaultError::Invariant(format!(
            "unsupported Projection Fault value version {}",
            value.value_version
        )));
    }
    if value.repo_id != expected_repo_id {
        return Err(ProjectionFaultError::Invariant(format!(
            "Projection Fault value RepoId {} differs from database RepoId {}",
            value.repo_id, expected_repo_id
        )));
    }
    if key_for(&value) != key {
        return Err(ProjectionFaultError::Invariant(
            "Projection Fault key/value identity mismatch".to_string(),
        ));
    }
    if value.status != ProjectionFaultStatus::Pending || value.retry_count == 0 {
        return Err(ProjectionFaultError::Invariant(
            "Projection Fault status/retry invariant is invalid".to_string(),
        ));
    }
    Ok(value)
}

fn same_identity(left: &DurableProjectionFault, right: &DurableProjectionFault) -> bool {
    left.repo_id == right.repo_id
        && left.fault_kind == right.fault_kind
        && left.origin == right.origin
        && left.target_path == right.target_path
        && left.source_path == right.source_path
        && left.doc_id == right.doc_id
}

fn key_for(value: &DurableProjectionFault) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(KEY_DOMAIN);
    hasher.update(value.repo_id.as_bytes());
    hasher.update([kind_tag(value.fault_kind)]);
    match &value.origin {
        ProjectionFaultOrigin::ProjectionPersistence => hasher.update([0]),
        ProjectionFaultOrigin::ProjectionRepair => hasher.update([1]),
        ProjectionFaultOrigin::RemoteImport {
            session_id,
            revision,
            request_id,
        } => {
            hasher.update([2]);
            hasher.update(session_id.to_be_bytes());
            hasher.update(revision.to_be_bytes());
            hasher.update(request_id.as_bytes());
        }
    }
    hash_optional_string(&mut hasher, value.target_path.as_deref());
    hash_optional_string(&mut hasher, value.source_path.as_deref());
    match value.doc_id {
        Some(doc_id) => {
            hasher.update([1]);
            hasher.update(doc_id.as_u128().to_be_bytes());
        }
        None => hasher.update([0]),
    }
    hasher.finalize().into()
}

fn kind_tag(kind: ProjectionFaultKind) -> u8 {
    match kind {
        ProjectionFaultKind::ProjectionWritebackFailed => 0,
        ProjectionFaultKind::ProjectionRebuildInterrupted => 1,
    }
}

fn hash_optional_string(hasher: &mut Sha256, value: Option<&str>) {
    match value {
        Some(value) => {
            hasher.update([1]);
            hasher.update((value.len() as u64).to_be_bytes());
            hasher.update(value.as_bytes());
        }
        None => hasher.update([0]),
    }
}

fn bounded_error(error: &str) -> String {
    if error.chars().count() <= MAX_ERROR_CHARS {
        return error.to_string();
    }
    let mut truncated = error.chars().take(MAX_ERROR_CHARS).collect::<String>();
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
mod tests;
