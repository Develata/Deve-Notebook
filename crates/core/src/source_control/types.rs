// crates\core\src\source_control
//! # Source Control 类型定义
//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout
//!   - 05_diff_logic#source-control-runtime
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChangeStatus {
    /// 已修改
    Modified,
    /// 新增
    Added,
    /// 已删除
    Deleted,
    /// 路径结构发生变化
    Renamed,
}

/// 用户可见变更所属的差异域。
///
/// `ConfirmedLedger` 表示内容已经进入 ledger authority，但尚未被最新
/// source-control commit anchor 覆盖；它不是 pending_fs_ops，也不能进入
/// staging/pending overlay。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChangeDomain {
    /// 外部文件/import 变化，由 pending_fs_ops 表达。
    #[default]
    WorkingDirectory,
    /// 已暂存的工作区变化。
    Staged,
    /// 已确认 ledger 变化，但尚未被 commit anchor 覆盖。
    ConfirmedLedger,
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
    /// 若该条目表示重命名候选，则记录其旧路径
    #[serde(default)]
    pub renamed_from: Option<String>,
    /// 绑定的稳定文档标识；纯新增文件在提交前允许为空
    #[serde(default)]
    pub doc_id: Option<DocId>,
    /// 变更状态
    pub status: ChangeStatus,
    /// 是否存在冲突或 External/Confirmed overlap blocker。
    ///
    /// pending_fs_ops 只持久化外部文件冲突；read projection 也会把
    /// staged/unstaged external changes 与 confirmed ledger dirty 的重叠
    /// 派生为 true，供 UI 禁用普通 Stage / Apply to Ledger。
    #[serde(default)]
    pub has_conflict: bool,
    /// 该条目所属差异域；旧客户端缺省解析为 Working Directory。
    #[serde(default)]
    pub domain: ChangeDomain,
    /// confirmed ledger diff 的基准 ledger seq；其它域为空。
    #[serde(default)]
    pub base_seq: Option<u64>,
    /// confirmed ledger diff 的目标 ledger seq；其它域为空。
    #[serde(default)]
    pub target_seq: Option<u64>,
}

/// 提交间文件差异
///
/// 表示两个提交之间单个文件的变更内容。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitFileDiff {
    /// 稳定文档身份；docless 结构残留或历史异常场景允许为空
    #[serde(default)]
    pub doc_id: Option<DocId>,
    /// 文件路径
    pub path: String,
    /// 变更状态 (Added / Modified / Deleted)
    pub status: ChangeStatus,
    /// 若该差异由结构路径变化触发，则记录旧路径
    #[serde(default)]
    pub previous_path: Option<String>,
    /// 旧版本内容 (commit_a 时刻)
    pub old_content: String,
    /// 新版本内容 (commit_b 时刻)
    pub new_content: String,
}

/// Exact identity used to re-open one file from a commit comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitFileDiffTarget {
    pub doc_id: DocId,
    pub path: String,
    #[serde(default)]
    pub previous_path: Option<String>,
    pub status: ChangeStatus,
}

/// Body-free commit comparison row used by Web history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitFileDiffSummary {
    pub doc_id: DocId,
    pub path: String,
    #[serde(default)]
    pub previous_path: Option<String>,
    pub status: ChangeStatus,
    pub target: CommitFileDiffTarget,
}
