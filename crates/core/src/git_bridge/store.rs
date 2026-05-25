//! plan_ref:
//!   - 03_storage#git-ecosystem-coexistence
//!   - 05_diff_logic#git-mirror-lifecycle
//!
//! Persistent Deve commit -> Git mirror state mapping.

use crate::models::RepoId;
use crate::source_control::CommitInfo;
use redb::{Database, ReadableTable, TableDefinition, TableError};

use super::error::{GitMirrorStoreError, GitMirrorStoreResult};
use super::failure_metadata::GitMirrorFailureMetadata;
mod schema;

pub use schema::{GitMirrorCommitState, GitMirrorFailureStage, GitMirrorRecord, GitMirrorSummary};

pub const GIT_MIRROR_COMMITS_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("git_mirror_commits");

pub fn init_table(db: &Database) -> GitMirrorStoreResult<()> {
    let write_txn = db.begin_write().map_err(|err| GitMirrorStoreError::Init {
        message: err.to_string(),
    })?;
    {
        let _ = write_txn
            .open_table(GIT_MIRROR_COMMITS_TABLE)
            .map_err(|err| GitMirrorStoreError::Init {
                message: err.to_string(),
            })?;
    }
    write_txn
        .commit()
        .map_err(|err| GitMirrorStoreError::Init {
            message: err.to_string(),
        })?;
    Ok(())
}

pub fn queue_deve_commit(
    db: &Database,
    repo_id: RepoId,
    commit: &CommitInfo,
) -> GitMirrorStoreResult<GitMirrorRecord> {
    if let Some(existing) = get_record(db, &commit.id)? {
        return Ok(existing);
    }
    let now = chrono::Utc::now().timestamp_millis();
    let record = GitMirrorRecord {
        deve_commit_id: commit.id.clone(),
        repo_id,
        ledger_seq: commit.ledger_seq,
        state: GitMirrorCommitState::Queued,
        git_commit_id: None,
        last_error: None,
        failure_stage: None,
        failure_subject: None,
        failure_command: None,
        failure_exit_status: None,
        queued_at_ms: now,
        updated_at_ms: now,
        attempts: 0,
    };
    write_record(db, &record)?;
    Ok(record)
}

pub fn mark_committed(
    db: &Database,
    deve_commit_id: &str,
    git_commit_id: &str,
) -> GitMirrorStoreResult<GitMirrorRecord> {
    let mut record = required_record(db, deve_commit_id)?;
    record.state = GitMirrorCommitState::Committed;
    record.git_commit_id = Some(git_commit_id.to_string());
    record.last_error = None;
    record.failure_stage = None;
    record.failure_subject = None;
    record.failure_command = None;
    record.failure_exit_status = None;
    record.attempts = record.attempts.saturating_add(1);
    record.updated_at_ms = chrono::Utc::now().timestamp_millis();
    write_record(db, &record)?;
    Ok(record)
}

pub fn mark_out_of_sync(
    db: &Database,
    deve_commit_id: &str,
    error: impl Into<String>,
) -> GitMirrorStoreResult<GitMirrorRecord> {
    let mut record = required_record(db, deve_commit_id)?;
    let error = error.into();
    let stage = GitMirrorFailureStage::classify(&error);
    let metadata = GitMirrorFailureMetadata::from_error(stage, &error);
    record.state = GitMirrorCommitState::OutOfSync;
    record.last_error = Some(error.clone());
    record.failure_stage = Some(stage);
    record.failure_subject = metadata.subject;
    record.failure_command = metadata.command;
    record.failure_exit_status = metadata.exit_status;
    record.attempts = record.attempts.saturating_add(1);
    record.updated_at_ms = chrono::Utc::now().timestamp_millis();
    write_record(db, &record)?;
    Ok(record)
}

pub fn get_record(
    db: &Database,
    deve_commit_id: &str,
) -> GitMirrorStoreResult<Option<GitMirrorRecord>> {
    let read_txn = db
        .begin_read()
        .map_err(|err| read_record_error(deve_commit_id, err))?;
    let table = match read_txn.open_table(GIT_MIRROR_COMMITS_TABLE) {
        Ok(table) => table,
        Err(TableError::TableDoesNotExist(_)) => return Ok(None),
        Err(err) => return Err(read_record_error(deve_commit_id, err)),
    };
    table
        .get(deve_commit_id)
        .map_err(|err| read_record_error(deve_commit_id, err))?
        .map(|guard| {
            serde_json::from_slice::<GitMirrorRecord>(guard.value()).map_err(|err| {
                GitMirrorStoreError::DecodeRecord {
                    deve_commit_id: deve_commit_id.to_string(),
                    message: err.to_string(),
                }
            })
        })
        .transpose()
}

pub fn list_records(db: &Database) -> GitMirrorStoreResult<Vec<GitMirrorRecord>> {
    let read_txn = db
        .begin_read()
        .map_err(|err| GitMirrorStoreError::ListRecords {
            message: err.to_string(),
        })?;
    let table = match read_txn.open_table(GIT_MIRROR_COMMITS_TABLE) {
        Ok(table) => table,
        Err(TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
        Err(err) => {
            return Err(GitMirrorStoreError::ListRecords {
                message: err.to_string(),
            });
        }
    };
    let mut records = Vec::new();
    let entries = table
        .iter()
        .map_err(|err| GitMirrorStoreError::ListRecords {
            message: err.to_string(),
        })?;
    for entry in entries {
        let (key, raw) = entry.map_err(|err| GitMirrorStoreError::ListRecords {
            message: err.to_string(),
        })?;
        let deve_commit_id = key.value().to_string();
        records.push(
            serde_json::from_slice::<GitMirrorRecord>(raw.value()).map_err(|err| {
                GitMirrorStoreError::DecodeRecord {
                    deve_commit_id,
                    message: err.to_string(),
                }
            })?,
        );
    }
    records.sort_by(|left, right| {
        left.ledger_seq
            .cmp(&right.ledger_seq)
            .then(left.deve_commit_id.cmp(&right.deve_commit_id))
    });
    Ok(records)
}

pub fn summarize_records(db: &Database) -> GitMirrorStoreResult<GitMirrorSummary> {
    let mut summary = GitMirrorSummary::default();
    for record in list_records(db)? {
        match record.state {
            GitMirrorCommitState::Queued => summary.queued += 1,
            GitMirrorCommitState::Committed => summary.committed += 1,
            GitMirrorCommitState::OutOfSync => summary.out_of_sync += 1,
        }
    }
    Ok(summary)
}

fn required_record(db: &Database, deve_commit_id: &str) -> GitMirrorStoreResult<GitMirrorRecord> {
    get_record(db, deve_commit_id)?.ok_or_else(|| GitMirrorStoreError::MissingRecord {
        deve_commit_id: deve_commit_id.to_string(),
    })
}

fn write_record(db: &Database, record: &GitMirrorRecord) -> GitMirrorStoreResult<()> {
    let bytes = serde_json::to_vec(record).map_err(|err| GitMirrorStoreError::EncodeRecord {
        deve_commit_id: record.deve_commit_id.clone(),
        message: err.to_string(),
    })?;
    let write_txn = db
        .begin_write()
        .map_err(|err| write_record_error(record, err))?;
    {
        let mut table = write_txn
            .open_table(GIT_MIRROR_COMMITS_TABLE)
            .map_err(|err| write_record_error(record, err))?;
        table
            .insert(record.deve_commit_id.as_str(), bytes.as_slice())
            .map_err(|err| write_record_error(record, err))?;
    }
    write_txn
        .commit()
        .map_err(|err| write_record_error(record, err))?;
    Ok(())
}

fn read_record_error(deve_commit_id: &str, err: impl std::fmt::Display) -> GitMirrorStoreError {
    GitMirrorStoreError::ReadRecord {
        deve_commit_id: deve_commit_id.to_string(),
        message: err.to_string(),
    }
}

fn write_record_error(
    record: &GitMirrorRecord,
    err: impl std::fmt::Display,
) -> GitMirrorStoreError {
    GitMirrorStoreError::WriteRecord {
        deve_commit_id: record.deve_commit_id.clone(),
        message: err.to_string(),
    }
}

#[cfg(test)]
mod tests;
