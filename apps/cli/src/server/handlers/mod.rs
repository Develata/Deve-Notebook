// apps\cli\src\server\handlers
//! 消息处理器模块
//!
//! 包含各类 ClientMessage 的处理逻辑，按功能领域划分。
pub mod document;
pub mod docs;
pub mod listing;
pub mod plugin;
pub mod search;
pub mod sync;
pub mod merge;
pub mod source_control;

use std::sync::Arc;
use crate::server::AppState;

/// 获取当前仓库的实际 RepoId
/// 
/// 用于替代硬编码的 `Uuid::nil()`
pub fn get_repo_id(state: &Arc<AppState>) -> uuid::Uuid {
    state.repo.get_repo_info()
        .ok()
        .flatten()
        .map(|info| info.uuid)
        .unwrap_or_else(uuid::Uuid::nil)
}
