// crates\core\src\source_control
//! # Source Control 模块
//!
//! 提供版本控制功能，包括暂存区管理、提交历史和变更检测。
//!
//! **模块结构**:
//! - `types`: 数据类型定义
//! - `staging`: 暂存区管理函数 [仅后端]
//! - `commits`: 提交管理函数 [仅后端]
//! - `changes`: 变更检测函数 [仅后端]

pub mod api;
pub mod diff;
pub mod types;

#[cfg(not(target_arch = "wasm32"))]
pub mod changes;
#[cfg(not(target_arch = "wasm32"))]
pub mod commits;
#[cfg(not(target_arch = "wasm32"))]
pub mod snapshot_paths;
#[cfg(not(target_arch = "wasm32"))]
pub mod staging;

// 重新导出常用类型
pub use api::SourceControlApi;
pub use types::{ChangeEntry, ChangeStatus, CommitInfo};

/// 提交时对快照的更新策略
pub enum SnapshotUpdate {
    /// 保存最新内容
    Save {
        doc_id: crate::models::DocId,
        path: String,
        content: String,
    },
    /// 删除快照 (表示文件被删除)
    Delete { doc_id: crate::models::DocId },
}
