// apps/cli/src/server/handlers/docs/mod.rs
//! # 文档 CRUD 处理器模块
//!
//! 将文档操作拆分为独立子模块，提高可维护性。
//!
//! ## 子模块
//! - `create`: 创建文档
//! - `rename`: 重命名/移动文档
//! - `delete`: 删除文档
//! - `copy`: 复制文档

mod copy;
mod copy_utils;
mod create;
mod create_file;
mod create_folder;
mod delete;
mod errors;
mod file_register;
mod node_helpers;
mod node_target;
mod rename;
mod rename_dir;
mod rename_file;

pub use copy::handle_copy_doc;
pub use create::handle_create_doc;
pub use delete::handle_delete_doc;
pub use rename::{handle_move_doc, handle_rename_doc};

use crate::server::channel::DualChannel;
use deve_core::models::RepoId;
use deve_core::protocol::ServerMessage;

/// 目录深度限制 (防止文件系统资源耗尽)
pub const MAX_DEPTH: usize = 10;

/// 验证路径是否符合安全规则
///
/// 检查项:
/// - 不包含 `..` (目录遍历)
/// - 不以 `/` 或 `\` 开头 (绝对路径)
/// - 目录深度不超过 `MAX_DEPTH`
pub fn validate_file_path(path: &str, ch: &DualChannel, scope_nonce: Option<u64>) -> bool {
    validate_path_kind(path, true, ch, scope_nonce)
}

pub fn validate_folder_path(path: &str, ch: &DualChannel, scope_nonce: Option<u64>) -> bool {
    validate_path_kind(path, false, ch, scope_nonce)
}

fn validate_path_kind(
    path: &str,
    allow_file_leaf: bool,
    ch: &DualChannel,
    scope_nonce: Option<u64>,
) -> bool {
    // 防止目录遍历攻击
    if path.contains("..") || path.starts_with('/') || path.starts_with('\\') {
        tracing::error!("路径校验失败 (遍历攻击): {}", path);
        errors::request_failed_scoped(ch, format!("Invalid path: {}", path), scope_nonce);
        return false;
    }

    // 检查目录深度
    if std::path::Path::new(path).components().count() > MAX_DEPTH {
        tracing::error!("路径校验失败 (深度超限): {}", path);
        errors::request_failed_scoped(
            ch,
            format!("Directory depth limit exceeded (max {})", MAX_DEPTH),
            scope_nonce,
        );
        return false;
    }

    let trimmed = path.trim_end_matches('/');
    let segments: Vec<_> = trimmed
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        errors::request_failed_scoped(ch, "Invalid empty path", scope_nonce);
        return false;
    }

    for (index, segment) in segments.iter().enumerate() {
        if *segment == ".notegit" {
            tracing::error!("路径校验失败 (保留目录): {}", path);
            errors::request_failed_scoped(
                ch,
                format!("Reserved internal path: {}", path),
                scope_nonce,
            );
            return false;
        }
        let is_leaf = index + 1 == segments.len();
        let md_dir = segment.ends_with(".md") && (!allow_file_leaf || !is_leaf);
        if md_dir {
            tracing::error!("路径校验失败 (.md 目录): {}", path);
            errors::request_failed_scoped(
                ch,
                format!("Markdown directory is forbidden: {}", path),
                scope_nonce,
            );
            return false;
        }
    }

    true
}

pub fn notify_fs_refresh(ch: &DualChannel, repo_id: RepoId, path: &str, change_type: &str) {
    ch.broadcast(ServerMessage::FsChangeDetected {
        repo_id: Some(repo_id),
        branch: None,
        scope_nonce: None,
        path: path.to_string(),
        change_type: change_type.to_string(),
        has_conflict: false,
    });
}
