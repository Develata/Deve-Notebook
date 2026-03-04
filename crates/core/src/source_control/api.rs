// crates/core/src/source_control/api.rs
//! # Source Control API (Trait)

use crate::source_control::{ChangeEntry, CommitInfo};
use anyhow::Result;

pub trait SourceControlApi: Send + Sync {
    // --- Pending (Working Directory) ---
    /// 获取所有待确认的文件变更 (Working Directory)
    fn list_pending_fs(&self) -> Result<Vec<ChangeEntry>>;
    /// 将待确认变更移入暂存区 (Working Dir → Staging)
    fn stage_pending(&self, path: &str) -> Result<()>;
    /// 丢弃待确认变更 (从 Working Dir 移除)
    fn discard_pending(&self, path: &str) -> Result<()>;

    // --- Staging & Commit ---
    fn list_changes(&self) -> Result<Vec<ChangeEntry>>;
    fn diff_doc_path(&self, path: &str) -> Result<String>;
    fn stage_file(&self, path: &str) -> Result<()>;
    fn commit_staged(&self, message: &str) -> Result<CommitInfo>;
}
