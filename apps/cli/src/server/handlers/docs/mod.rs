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
mod delete;
mod node_helpers;
mod rename;

pub use copy::handle_copy_doc;
pub use create::handle_create_doc;
pub use delete::handle_delete_doc;
pub use rename::{handle_move_doc, handle_rename_doc};

use crate::server::channel::DualChannel;

/// 目录深度限制 (防止文件系统资源耗尽)
pub const MAX_DEPTH: usize = 10;

/// 验证路径是否符合安全规则
///
/// 检查项:
/// - 不包含 `..` (目录遍历)
/// - 不以 `/` 或 `\` 开头 (绝对路径)
/// - 目录深度不超过 `MAX_DEPTH`
pub fn validate_path(path: &str, ch: &DualChannel) -> bool {
    // 防止目录遍历攻击
    if path.contains("..") || path.starts_with('/') || path.starts_with('\\') {
        tracing::error!("路径校验失败 (遍历攻击): {}", path);
        ch.send_error(format!("Invalid path: {}", path));
        return false;
    }

    // 检查目录深度
    if std::path::Path::new(path).components().count() > MAX_DEPTH {
        tracing::error!("路径校验失败 (深度超限): {}", path);
        ch.send_error(format!(
            "Directory depth limit exceeded (max {})",
            MAX_DEPTH
        ));
        return false;
    }

    true
}
