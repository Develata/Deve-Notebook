// apps\cli\src\server\handlers
//! 消息处理器模块
//!
//! 包含各类 ClientMessage 的处理逻辑，按功能领域划分。
pub mod docs;
pub mod document;
pub mod key_exchange;
pub mod listing;
pub mod merge;
pub mod plugin;
pub mod repo;
pub mod search;
pub mod source_control;
pub mod switcher;
pub mod sync;

use crate::server::AppState;
use std::sync::Arc;

/// 获取当前仓库的实际 RepoId
///
/// 用于替代硬编码的 `Uuid::nil()`
pub fn get_repo_id(state: &Arc<AppState>) -> uuid::Uuid {
    state
        .repo
        .get_repo_info()
        .ok()
        .flatten()
        .map(|info| info.uuid)
        .unwrap_or_else(uuid::Uuid::nil)
}
