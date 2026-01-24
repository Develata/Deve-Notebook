// crates\core\src\ledger\merge\types.rs
// ---------------------------------------------------------------
// 模块：三路合并类型定义
// 作用：为合并引擎提供统一的数据结构
// 功能：合并结果与冲突片段的结构化描述
// ---------------------------------------------------------------

use serde::{Deserialize, Serialize};

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

/// 冲突片段（基于行范围）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConflictHunk {
    pub start_line: usize,
    pub length: usize,
    pub local_lines: Vec<String>,
    pub remote_lines: Vec<String>,
}
