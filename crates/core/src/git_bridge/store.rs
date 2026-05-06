//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!
//! Persistent Deve commit -> Git mirror state mapping.

use crate::models::RepoId;
use crate::source_control::CommitInfo;
use anyhow::{Result, anyhow};
use redb::{Database, ReadableTable, TableDefinition, TableError};
use serde::{Deserialize, Serialize};

use super::failure_metadata::GitMirrorFailureMetadata;

pub const GIT_MIRROR_COMMITS_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("git_mirror_commits");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitMirrorCommitState {
    Queued,
    Committed,
    OutOfSync,
}

impl GitMirrorCommitState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Committed => "committed",
            Self::OutOfSync => "out_of_sync",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitMirrorFailureStage {
    MirrorNotReady,
    DeveSourceControl,
    NotegitProtection,
    ProjectionScope,
    GitHistoryMapping,
    GitWorktree,
    GitCommand,
    MirrorExecutor,
}

impl GitMirrorFailureStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MirrorNotReady => "mirror_not_ready",
            Self::DeveSourceControl => "deve_source_control",
            Self::NotegitProtection => "notegit_protection",
            Self::ProjectionScope => "projection_scope",
            Self::GitHistoryMapping => "git_history_mapping",
            Self::GitWorktree => "git_worktree",
            Self::GitCommand => "git_command",
            Self::MirrorExecutor => "mirror_executor",
        }
    }

    pub fn classify(error: &str) -> Self {
        let normalized = error.to_ascii_lowercase();
        if normalized.contains("not ready:")
            || normalized.contains("protectionmissing")
            || normalized.contains("does not ignore .notegit")
        {
            return Self::MirrorNotReady;
        }
        if normalized.contains("pending source-control")
            || normalized.contains("staged source-control")
            || normalized.contains("pending_fs")
            || normalized.contains("staging")
        {
            return Self::DeveSourceControl;
        }
        if normalized.contains("outside queued deve commit")
            || normalized.contains("outside current deve projection snapshot")
            || normalized.contains("projection diff")
        {
            return Self::ProjectionScope;
        }
        if normalized.contains(".notegit") || normalized.contains("tracked by git") {
            return Self::NotegitProtection;
        }
        if normalized.contains("unsafe projection path") {
            return Self::ProjectionScope;
        }
        if normalized.contains("parent")
            || normalized.contains("git head does not match")
            || normalized.contains("requires empty git history")
            || normalized.contains("not mirrored")
        {
            return Self::GitHistoryMapping;
        }
        if normalized.contains("worktree") || normalized.contains("rev-parse") {
            return Self::GitWorktree;
        }
        if normalized.starts_with("git mirror ") {
            return Self::MirrorExecutor;
        }
        if normalized.starts_with("git ")
            || normalized.starts_with("failed to run git ")
            || normalized.contains(" for git ")
            || normalized.contains(" stdin for git ")
        {
            return Self::GitCommand;
        }
        Self::MirrorExecutor
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitMirrorRecord {
    pub deve_commit_id: String,
    pub repo_id: RepoId,
    pub ledger_seq: u64,
    pub state: GitMirrorCommitState,
    #[serde(default)]
    pub git_commit_id: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub failure_stage: Option<GitMirrorFailureStage>,
    #[serde(default)]
    pub failure_subject: Option<String>,
    #[serde(default)]
    pub failure_command: Option<String>,
    #[serde(default)]
    pub failure_exit_status: Option<String>,
    pub queued_at_ms: i64,
    pub updated_at_ms: i64,
    #[serde(default)]
    pub attempts: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitMirrorSummary {
    pub queued: usize,
    pub committed: usize,
    pub out_of_sync: usize,
}

pub fn init_table(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(GIT_MIRROR_COMMITS_TABLE)?;
    }
    write_txn.commit()?;
    Ok(())
}

pub fn queue_deve_commit(
    db: &Database,
    repo_id: RepoId,
    commit: &CommitInfo,
) -> Result<GitMirrorRecord> {
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
) -> Result<GitMirrorRecord> {
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
) -> Result<GitMirrorRecord> {
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

pub fn get_record(db: &Database, deve_commit_id: &str) -> Result<Option<GitMirrorRecord>> {
    let read_txn = db.begin_read()?;
    let table = match read_txn.open_table(GIT_MIRROR_COMMITS_TABLE) {
        Ok(table) => table,
        Err(TableError::TableDoesNotExist(_)) => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    table
        .get(deve_commit_id)?
        .map(|guard| serde_json::from_slice::<GitMirrorRecord>(guard.value()).map_err(Into::into))
        .transpose()
}

pub fn list_records(db: &Database) -> Result<Vec<GitMirrorRecord>> {
    let read_txn = db.begin_read()?;
    let table = match read_txn.open_table(GIT_MIRROR_COMMITS_TABLE) {
        Ok(table) => table,
        Err(TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };
    let mut records = Vec::new();
    for entry in table.iter()? {
        let (_, raw) = entry?;
        records.push(serde_json::from_slice::<GitMirrorRecord>(raw.value())?);
    }
    records.sort_by(|left, right| {
        left.ledger_seq
            .cmp(&right.ledger_seq)
            .then(left.deve_commit_id.cmp(&right.deve_commit_id))
    });
    Ok(records)
}

pub fn summarize_records(db: &Database) -> Result<GitMirrorSummary> {
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

fn required_record(db: &Database, deve_commit_id: &str) -> Result<GitMirrorRecord> {
    get_record(db, deve_commit_id)?
        .ok_or_else(|| anyhow!("Git mirror record not found for Deve commit {deve_commit_id}"))
}

fn write_record(db: &Database, record: &GitMirrorRecord) -> Result<()> {
    let bytes = serde_json::to_vec(record)?;
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(GIT_MIRROR_COMMITS_TABLE)?;
        table.insert(record.deve_commit_id.as_str(), bytes.as_slice())?;
    }
    write_txn.commit()?;
    Ok(())
}

#[cfg(test)]
#[path = "store_test.rs"]
mod tests;
