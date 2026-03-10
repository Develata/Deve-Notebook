//! # Pending Rename 辅助
//!
//! Invariants:
//! - 外部 rename/move 在用户 Stage/Commit 前只能表现为 pending 候选。
//! - 同一 `doc_id` 的 rename 候选必须同时保留旧路径删除项与新路径新增项。

use super::pending;
use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::source_control::ChangeStatus;
use anyhow::Result;
use std::sync::Arc;

pub(super) fn upsert_external_rename(
    repo: &Arc<RepoManager>,
    repo_name: &str,
    old_path: &str,
    new_path: &str,
    doc_id: DocId,
) -> Result<()> {
    pending::upsert(
        repo,
        repo_name,
        old_path,
        ChangeStatus::Deleted,
        Some(doc_id),
        None,
    )?;
    pending::upsert(
        repo,
        repo_name,
        new_path,
        ChangeStatus::Added,
        Some(doc_id),
        Some(old_path),
    )
}
