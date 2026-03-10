// crates\core\src\source_control
//! # Source Control 类型定义
//!
//! 定义版本控制相关的数据结构，用于暂存区和提交历史。

use crate::models::DocId;
use serde::{Deserialize, Serialize};

/// 提交信息结构体
///
/// 对应 Git 的 commit 概念，包含提交元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    /// 提交 ID (UUID)
    pub id: String,
    /// 父提交 ID（构成提交链，首次提交为 None）
    #[serde(default)]
    pub parent_id: Option<String>,
    /// 提交消息
    pub message: String,
    /// 提交时间戳 (毫秒)
    pub timestamp: i64,
    /// 包含的文档数量
    pub doc_count: u32,
    /// 对应的 Ledger 全局序列号 (Anchor Point)
    pub ledger_seq: u64,
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

/// 冲突解决策略
///
/// 当文件同时在 FS 和 Ledger 中有未提交变更时，用户可选择保留哪一方。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// 保留文件系统版本 (FS 覆盖 Ledger)
    KeepFs,
    /// 保留 Ledger 版本 (Ledger 写回磁盘)
    KeepLedger,
}

/// 变更条目
///
/// 表示单个文件的变更信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEntry {
    /// 文件路径
    pub path: String,
    /// 绑定的稳定文档标识；纯新增文件在提交前允许为空
    #[serde(default)]
    pub doc_id: Option<DocId>,
    /// 变更状态
    pub status: ChangeStatus,
    /// 是否存在冲突 (仅 unstaged 条目可能为 true)
    #[serde(default)]
    pub has_conflict: bool,
}

/// 提交间文件差异
///
/// 表示两个提交之间单个文件的变更内容。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitFileDiff {
    /// 文件路径
    pub path: String,
    /// 变更状态 (Added / Modified / Deleted)
    pub status: ChangeStatus,
    /// 旧版本内容 (commit_a 时刻)
    pub old_content: String,
    /// 新版本内容 (commit_b 时刻)
    pub new_content: String,
}
