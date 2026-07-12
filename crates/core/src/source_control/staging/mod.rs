// crates\core\src\source_control
//! # 暂存区管理 (Staging Manager)
//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout
//!   - 05_diff_logic#source-control-runtime
//!
//! 管理文件的暂存状态，持久化到数据库。
//!
//! **存储结构**:
//! - Table: `staged_files` - 存储已暂存的文件路径及其变更元数据

mod index;
mod query;
mod target;

use crate::models::DocId;
use crate::source_control::ChangeStatus;
use crate::source_control::pending_fs::PendingFsEntry;
use anyhow::Result;
pub use query::{
    get_staged, is_staged, list_staged, list_staged_entries, list_staged_entries_for_doc,
    list_staged_with_status,
};
use redb::{Database, ReadableTable, TableDefinition, WriteTransaction};
use serde::{Deserialize, Serialize};
pub use target::{get_staged_for_target, take_staged_for_target};

/// 暂存区表定义 (path -> JSON bytes)
pub const STAGED_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("staged_files");

/// 暂存条目（包含变更状态）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagedEntry {
    pub timestamp: i64,
    #[serde(default)]
    pub renamed_from: Option<String>,
    #[serde(default)]
    pub doc_id: Option<DocId>,
    pub status: ChangeStatus,
    pub content_hash: String,
    pub has_conflict: bool,
    #[serde(default)]
    pub resolved_conflict: bool,
}

/// 初始化暂存区表
pub fn init_table(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let _ = write_txn.open_table(STAGED_TABLE)?;
        index::init_table(&write_txn)?;
    }
    write_txn.commit()?;
    Ok(())
}

/// 将一条 pending 记录整体移入暂存区。
pub fn stage_pending_entry(db: &Database, entry: &PendingFsEntry) -> Result<()> {
    let staged = staged_from_pending(entry, chrono::Utc::now().timestamp_millis(), false);
    stage_entry(db, &entry.path, &staged)
}

pub fn stage_resolved_pending_entry(db: &Database, entry: &PendingFsEntry) -> Result<()> {
    let staged = staged_from_pending(entry, chrono::Utc::now().timestamp_millis(), true);
    stage_entry(db, &entry.path, &staged)
}

/// Atomically move an exact batch from pending into staging.
pub(crate) fn stage_pending_entries_atomically(
    db: &Database,
    entries: &[PendingFsEntry],
    resolved_conflict: bool,
) -> Result<()> {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let write_txn = db.begin_write()?;
    for entry in entries {
        crate::source_control::pending_fs::remove_exact_in_txn(&write_txn, entry)?;
        let staged = staged_from_pending(entry, timestamp, resolved_conflict);
        stage_entry_in_txn(&write_txn, &entry.path, &staged)?;
    }
    write_txn.commit()?;
    Ok(())
}

fn staged_from_pending(
    entry: &PendingFsEntry,
    timestamp: i64,
    resolved_conflict: bool,
) -> StagedEntry {
    StagedEntry {
        timestamp,
        renamed_from: entry.renamed_from.clone(),
        doc_id: entry.doc_id,
        status: entry.change_type,
        content_hash: entry.content_hash.clone(),
        has_conflict: if resolved_conflict {
            false
        } else {
            entry.has_conflict
        },
        resolved_conflict,
    }
}

/// 移除并返回单条暂存记录
pub fn take_staged(db: &Database, path: &str) -> Result<Option<StagedEntry>> {
    let existing = get_staged(db, path)?;
    if existing.is_none() {
        return Ok(None);
    }
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(STAGED_TABLE)?;
        index::remove(
            &write_txn,
            existing.as_ref().and_then(|entry| entry.doc_id),
            path,
        )?;
        table.remove(path)?;
    }
    write_txn.commit()?;
    Ok(existing)
}

fn stage_entry(db: &Database, path: &str, entry: &StagedEntry) -> Result<()> {
    let write_txn = db.begin_write()?;
    stage_entry_in_txn(&write_txn, path, entry)?;
    write_txn.commit()?;
    tracing::info!("Staged file: {} ({:?})", path, entry.status);
    Ok(())
}

fn stage_entry_in_txn(write_txn: &WriteTransaction, path: &str, entry: &StagedEntry) -> Result<()> {
    let bytes = serde_json::to_vec(entry)?;
    let mut table = write_txn.open_table(STAGED_TABLE)?;
    let previous = table
        .get(path)?
        .map(|guard| serde_json::from_slice::<StagedEntry>(guard.value()))
        .transpose()?;
    index::replace(
        write_txn,
        previous.as_ref().and_then(|item| item.doc_id),
        entry.doc_id,
        path,
    )?;
    table.insert(path, bytes.as_slice())?;
    Ok(())
}

pub(crate) fn unstage_target_atomically(
    db: &Database,
    target: &crate::protocol::ScPathTarget,
    detected_at: i64,
) -> Result<bool> {
    let Some(expected) = target::get_staged_for_unstage_target(db, target)? else {
        return Ok(false);
    };
    unstage_expected_atomically(db, target, &expected, detected_at, || Ok(()))?;
    Ok(true)
}

fn unstage_expected_atomically<F>(
    db: &Database,
    target: &crate::protocol::ScPathTarget,
    expected: &(String, StagedEntry),
    detected_at: i64,
    after_staged_remove: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    let write_txn = db.begin_write()?;
    let current = target::get_staged_for_unstage_target_in_txn(&write_txn, target)?;
    match current {
        Some(ref current) if current == expected => {}
        Some(_) => anyhow::bail!("Staged entry changed before Unstage: {}", expected.0),
        None => anyhow::bail!("Staged entry disappeared before Unstage: {}", expected.0),
    }
    consume_exact_in_txn(&write_txn, std::slice::from_ref(expected))?;
    after_staged_remove()?;
    let (path, staged) = expected;
    crate::source_control::pending_fs::restore_unstaged_in_txn(
        &write_txn,
        &PendingFsEntry {
            path: path.clone(),
            renamed_from: staged.renamed_from.clone(),
            doc_id: staged.doc_id,
            change_type: staged.status,
            content_hash: staged.content_hash.clone(),
            detected_at,
            has_conflict: staged.has_conflict,
        },
    )?;
    write_txn.commit()?;
    Ok(())
}

pub(crate) fn clear_in_txn(write_txn: &WriteTransaction) -> Result<()> {
    write_txn.delete_table(STAGED_TABLE)?;
    let _ = write_txn.open_table(STAGED_TABLE)?;
    write_txn.delete_multimap_table(crate::ledger::schema::STAGED_DOC_INDEX)?;
    index::init_table(write_txn)
}

pub(crate) fn consume_exact_in_txn(
    write_txn: &WriteTransaction,
    expected: &[(String, StagedEntry)],
) -> Result<()> {
    let mut table = write_txn.open_table(STAGED_TABLE)?;
    for (path, expected_entry) in expected {
        let current = table
            .get(path.as_str())?
            .map(|guard| serde_json::from_slice::<StagedEntry>(guard.value()))
            .transpose()?;
        match current {
            Some(current) if current == *expected_entry => {}
            Some(_) => anyhow::bail!("Staged entry changed before Apply to Ledger: {}", path),
            None => anyhow::bail!("Staged entry disappeared before Apply to Ledger: {}", path),
        }
    }
    for (path, entry) in expected {
        index::remove(write_txn, entry.doc_id, path)?;
        table.remove(path.as_str())?;
    }
    Ok(())
}

/// 清空暂存区 (提交后调用)
pub fn clear(db: &Database) -> Result<()> {
    let write_txn = db.begin_write()?;
    clear_in_txn(&write_txn)?;
    write_txn.commit()?;
    tracing::info!("Cleared staging area");
    Ok(())
}

#[cfg(test)]
mod atomic_tests {
    use super::*;
    use crate::protocol::ScPathTarget;
    use crate::source_control::pending_fs;

    fn pending(path: &str) -> PendingFsEntry {
        PendingFsEntry {
            path: path.into(),
            renamed_from: None,
            doc_id: None,
            change_type: ChangeStatus::Added,
            content_hash: path.into(),
            detected_at: 1,
            has_conflict: false,
        }
    }

    fn target(path: &str, doc_id: Option<DocId>) -> ScPathTarget {
        ScPathTarget {
            path: path.into(),
            doc_id,
            domain: None,
        }
    }

    #[test]
    fn batch_stage_rolls_back_if_any_pending_entry_changed_or_disappeared() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let db = Database::create(dir.path().join("stage-atomic.redb"))?;
        pending_fs::init_table(&db)?;
        init_table(&db)?;
        let first = pending("notes/first.md");
        let missing = pending("notes/missing.md");
        pending_fs::upsert(&db, &first)?;

        let error = stage_pending_entries_atomically(&db, &[first.clone(), missing], false)
            .expect_err("missing second entry must abort the whole batch");

        assert!(error.to_string().contains("disappeared before stage"));
        assert!(pending_fs::get(&db, &first.path)?.is_some());
        assert!(get_staged(&db, &first.path)?.is_none());
        Ok(())
    }

    #[test]
    fn apply_snapshot_consume_rejects_replaced_staged_entry_without_removal() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let db = Database::create(dir.path().join("apply-stage-snapshot.redb"))?;
        init_table(&db)?;
        let original_pending = pending("notes/a.md");
        stage_pending_entry(&db, &original_pending)?;
        let original = get_staged(&db, &original_pending.path)?.expect("original staged entry");

        let mut replacement_pending = original_pending.clone();
        replacement_pending.content_hash = "replacement".into();
        stage_pending_entry(&db, &replacement_pending)?;

        let write_txn = db.begin_write()?;
        let error = consume_exact_in_txn(&write_txn, &[(original_pending.path.clone(), original)])
            .expect_err("replacement must fail exact staged snapshot consumption");
        assert!(error.to_string().contains("changed before Apply"));
        drop(write_txn);
        assert_eq!(
            get_staged(&db, &original_pending.path)?
                .expect("replacement remains")
                .content_hash,
            "replacement"
        );
        Ok(())
    }

    #[test]
    fn apply_snapshot_consume_preserves_new_unrelated_staging() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let db = Database::create(dir.path().join("apply-stage-new-entry.redb"))?;
        init_table(&db)?;
        let expected_pending = pending("notes/expected.md");
        let new_pending = pending("notes/new.md");
        stage_pending_entry(&db, &expected_pending)?;
        let expected = get_staged(&db, &expected_pending.path)?.expect("expected staged entry");
        stage_pending_entry(&db, &new_pending)?;

        let write_txn = db.begin_write()?;
        consume_exact_in_txn(&write_txn, &[(expected_pending.path.clone(), expected)])?;
        write_txn.commit()?;

        assert!(get_staged(&db, &expected_pending.path)?.is_none());
        assert!(get_staged(&db, &new_pending.path)?.is_some());
        Ok(())
    }

    #[test]
    fn unstage_second_step_failure_rolls_back_staged_remove_and_indexes() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let db = Database::create(dir.path().join("unstage-second-step.redb"))?;
        pending_fs::init_table(&db)?;
        init_table(&db)?;
        let doc_id = DocId::new();
        let mut entry = pending("notes/a.md");
        entry.doc_id = Some(doc_id);
        stage_pending_entry(&db, &entry)?;
        let expected =
            target::get_staged_for_unstage_target(&db, &target(&entry.path, Some(doc_id)))?
                .expect("staged entry");

        let error = unstage_expected_atomically(
            &db,
            &target(&entry.path, Some(doc_id)),
            &expected,
            2,
            || anyhow::bail!("injected pending write failure"),
        )
        .expect_err("second step failure must abort transaction");

        assert!(error.to_string().contains("injected pending write failure"));
        assert_eq!(get_staged(&db, &entry.path)?, Some(expected.1));
        assert!(pending_fs::get(&db, &entry.path)?.is_none());
        assert_eq!(list_staged_entries_for_doc(&db, doc_id)?.len(), 1);
        assert!(pending_fs::list_for_doc(&db, doc_id)?.is_empty());
        Ok(())
    }

    #[test]
    fn unstage_rejects_concurrently_replaced_staged_entry() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let db = Database::create(dir.path().join("unstage-replaced.redb"))?;
        pending_fs::init_table(&db)?;
        init_table(&db)?;
        let entry = pending("notes/a.md");
        stage_pending_entry(&db, &entry)?;
        let requested = target(&entry.path, None);
        let expected =
            target::get_staged_for_unstage_target(&db, &requested)?.expect("original staged entry");
        let mut replacement = entry.clone();
        replacement.content_hash = "replacement".into();
        stage_pending_entry(&db, &replacement)?;

        let error = unstage_expected_atomically(&db, &requested, &expected, 2, || Ok(()))
            .expect_err("replacement must fail exact comparison");

        assert!(error.to_string().contains("changed before Unstage"));
        assert_eq!(
            get_staged(&db, &entry.path)?
                .expect("replacement remains")
                .content_hash,
            "replacement"
        );
        assert!(pending_fs::get(&db, &entry.path)?.is_none());
        Ok(())
    }

    #[test]
    fn unstage_rejects_newer_pending_without_overwriting_evidence() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let db = Database::create(dir.path().join("unstage-newer-pending.redb"))?;
        pending_fs::init_table(&db)?;
        init_table(&db)?;
        let doc_id = DocId::new();
        let mut staged_source = pending("notes/a.md");
        staged_source.doc_id = Some(doc_id);
        stage_pending_entry(&db, &staged_source)?;

        let mut newer_pending = staged_source.clone();
        newer_pending.content_hash = "newer-watcher-hash".into();
        newer_pending.detected_at = 2;
        newer_pending.has_conflict = true;
        pending_fs::upsert(&db, &newer_pending)?;

        let requested = target(&staged_source.path, Some(doc_id));
        let error = unstage_target_atomically(&db, &requested, 3)
            .expect_err("newer pending evidence must make Unstage fail closed");

        assert!(
            error
                .to_string()
                .contains("Pending FS destination changed before Unstage")
        );
        assert!(get_staged(&db, &staged_source.path)?.is_some());
        let preserved = pending_fs::get(&db, &staged_source.path)?.expect("newer pending remains");
        assert!(pending_fs::semantic_eq(&preserved, &newer_pending));
        assert_eq!(preserved.detected_at, newer_pending.detected_at);
        assert_eq!(list_staged_entries_for_doc(&db, doc_id)?.len(), 1);
        assert_eq!(pending_fs::list_for_doc(&db, doc_id)?.len(), 1);
        Ok(())
    }
}
