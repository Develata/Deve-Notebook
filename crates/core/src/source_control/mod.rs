// crates\core\src\source_control
//! # Source Control 模块
//! plan_ref:
//!   - 04_storage#repo-runtime-layout
//!   - 07_diff_logic#source-control-runtime
//!
//! 提供版本控制功能，包括暂存区管理、提交历史和变更检测。
//!
//! **模块结构**:
//! - `types`: 数据类型定义
//! - `staging`: 暂存区管理函数 [仅后端]
//! - `commits`: 提交管理函数 [仅后端]
//! - `changes`: 变更检测函数 [仅后端]
//! - `pending_fs`: 待确认文件变更管理 (Working Directory) [仅后端]
//! - `commit_diff`: 提交间差异计算 [仅后端]

#[cfg(not(target_arch = "wasm32"))]
pub mod api;
pub mod diff;
pub mod line_diff;
pub mod types;

#[cfg(not(target_arch = "wasm32"))]
pub mod changes;
#[cfg(not(target_arch = "wasm32"))]
pub mod commit_diff;
#[cfg(not(target_arch = "wasm32"))]
mod commit_diff_error;
#[cfg(not(target_arch = "wasm32"))]
mod commit_diff_paths;
#[cfg(not(target_arch = "wasm32"))]
pub mod commits;
#[cfg(not(target_arch = "wasm32"))]
pub mod conflict;
#[cfg(not(target_arch = "wasm32"))]
pub mod pending_fs;
#[cfg(not(target_arch = "wasm32"))]
pub mod staging;

// 重新导出常用类型
#[cfg(not(target_arch = "wasm32"))]
pub use api::SourceControlApi;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use commit_diff_error::CommitDiffError;
pub use line_diff::ChangeRange;
pub use types::{ChangeEntry, ChangeStatus, CommitFileDiff, CommitInfo, ConflictResolution};
