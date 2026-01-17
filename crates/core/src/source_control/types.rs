// crates\core\src\source_control
//! # Source Control 类型定义
//!
//! 定义版本控制相关的数据结构，用于暂存区和提交历史。

use serde::{Deserialize, Serialize};

/// 提交信息结构体
///
/// 对应 Git 的 commit 概念，包含提交元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    /// 提交 ID (UUID)
    pub id: String,
    /// 提交消息
    pub message: String,
    /// 提交时间戳 (毫秒)
    pub timestamp: i64,
    /// 包含的文档数量
    pub doc_count: u32,
}

/// 文件变更状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeStatus {
    /// 已修改
    Modified,
    /// 新增
    Added,
    /// 已删除
    Deleted,
}

/// 变更条目
///
/// 表示单个文件的变更信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEntry {
    /// 文件路径
    pub path: String,
    /// 变更状态
    pub status: ChangeStatus,
}
