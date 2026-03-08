// crates\core\src
//! # 虚拟文件系统
//!
//! 本模块提供 `Vfs` 结构体用于管理 vault 目录。
//!
//! ## 功能
//!
//! - `get_inode`: 获取跨平台文件标识符，用于重命名检测
//!
//! VFS 层仅保留文件系统标识抽象，repo-aware 扫描逻辑已迁移到 `sync::scan`。

use crate::models::FileNodeId;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct Vfs {
    pub root: PathBuf,
}

impl Vfs {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        let abs_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        Self { root: abs_root }
    }

    pub fn get_inode(&self, rel_path: &str) -> Result<Option<FileNodeId>> {
        let full_path = self.root.join(rel_path);
        if !full_path.exists() {
            return Ok(None);
        }

        let file_id = file_id::get_file_id(&full_path)?;

        // Hash the FileId to get a stable u128 for Redb
        use crate::utils::hash::StableHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = StableHasher::new();
        file_id.hash(&mut hasher);
        let hash = hasher.finish();
        // FileId hash is u64. We can use it. FileNodeId wraps u128.

        Ok(Some(FileNodeId { id: hash as u128 }))
    }
}
