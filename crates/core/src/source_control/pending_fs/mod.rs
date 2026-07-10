// crates/core/src/source_control/pending_fs.rs
//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!
//! # 待确认文件变更管理 (Pending FS Ops)
//!
//! 存储 Watcher 检测到但用户尚未确认的文件系统变更。
//! 这是 Git-like 三阶段工作流的 Working Directory 层。
//!
//! **不变量**: pending_fs_ops 中的条目永远不会自动进入 Ledger，
//! 必须经过用户显式 Stage → Commit 才会生成 Op。
//!
//! **Idempotency Invariant (幂等性不变量)**:
//! 重复信号触发同一 `(path, status, hash, ...)` 的 upsert **MUST** 产生
//! 字节相同的 side table 行。`detected_at` 仅在语义字段变化时更新；
//! 若所有语义字段与已存在条目相等，`upsert` **MUST** 跳过写入，
//! 保持原行（含原 `detected_at`）不变。详见 plan 04_storage#watcher-contract。
//!
//! **存储结构**:
//! - Table: `pending_fs_ops` (path -> PendingFsEntry 序列化字节)

mod index;
mod mutation;
mod query;
mod target;

#[cfg(test)]
mod idempotency_test;

use crate::models::DocId;
use crate::source_control::ChangeStatus;
pub(crate) use mutation::remove_exact_in_txn;
pub(crate) use mutation::semantic_eq;
pub use mutation::{clear, init_table, move_for_doc, remove, remove_subtree, upsert, upsert_many};
pub use query::{get, list_all, list_for_doc};
use serde::{Deserialize, Serialize};
pub use target::{get_for_target, take_for_target};

/// 待确认的文件变更条目
///
/// **Pre-condition**: path 已经过 `to_forward_slash` 规范化。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingFsEntry {
    /// 相对路径 (forward-slash)
    pub path: String,
    /// 若该条目由 rename/move 产生，则记录旧路径
    #[serde(default)]
    pub renamed_from: Option<String>,
    /// 与当前候选绑定的稳定文档标识；纯新增文件允许为空
    #[serde(default)]
    pub doc_id: Option<DocId>,
    /// 变更类型
    pub change_type: ChangeStatus,
    /// 内容 SipHash 指纹 (64-bit, hex 编码；非密码学安全)
    pub content_hash: String,
    /// 检测时间戳 (毫秒)
    pub detected_at: i64,
    /// 是否存在冲突 (FS 与 Ledger 均有未提交变更)
    #[serde(default)]
    pub has_conflict: bool,
}

/// 计算内容的 SipHash 指纹 (64-bit, hex 编码)
///
/// 用于快速判断文件内容是否真正变化（防抖 / 冲突检测）。
/// 注意：非密码学安全哈希，仅用于本地去重。
pub fn content_hash(content: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
