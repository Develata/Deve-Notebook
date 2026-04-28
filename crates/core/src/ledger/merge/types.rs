// crates\core\src\ledger\merge\types.rs
//! plan_ref:
//!   - 04_storage#facts-partition
//!   - 03_rendering#document-authority-bridge
//!
// ---------------------------------------------------------------
// 模块：三路合并类型定义
// 作用：为合并引擎提供统一的数据结构
// 功能：合并结果与冲突片段的结构化描述
// ---------------------------------------------------------------

use serde::{Deserialize, Serialize};

pub use crate::merge::ConflictHunk;

/// 合并操作结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MergeResult {
    /// 自动合并成功
    Success(String),
    /// 发生冲突
    Conflict {
        base: String,
        local: String,
        remote: String,
        conflicts: Vec<ConflictHunk>,
    },
}
