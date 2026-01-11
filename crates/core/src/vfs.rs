//! # 虚拟文件系统
//!
//! 本模块提供 `Vfs` 结构体用于管理 vault 目录。
//!
//! ## 功能
//!
//! - `get_inode`: 获取跨平台文件标识符，用于重命名检测
//! - `scan`: 同步 Ledger 与文件系统（添加新文件、清理幽灵条目）
//!
//! VFS 层抽象了文件系统操作，提供在文件重命名后仍保持稳定的标识符。

use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::ledger::Ledger;
use crate::models::FileNodeId;


pub struct Vfs {
    pub root: PathBuf,
}

impl Vfs {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        let abs_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        Self {
            root: abs_root,
        }
    }

    pub fn get_inode(&self, rel_path: &str) -> Result<Option<FileNodeId>> {
        let full_path = self.root.join(rel_path);
        if !full_path.exists() {
            return Ok(None);
        }
        
        let file_id = file_id::get_file_id(&full_path)?;
        
        // Hash the FileId to get a stable u128 for Redb
        // This is a simplification. Ideally we should serialize FileId.
        // For Phase 0, we use a simple hash.
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        file_id.hash(&mut hasher);
        let hash = hasher.finish(); 
        // FileId hash is u64. We can use it. FileNodeId wraps u128.
        
        Ok(Some(FileNodeId { id: hash as u128 }))
    }

    /// Scan the vault directory and ensure every markdown file has a DocId in the Ledger.
    /// Also removes entries from the Ledger that no longer exist on disk.
    pub fn scan(&self, ledger: &Ledger) -> Result<usize> {
        let mut count = 0;
        let mut on_disk_paths = std::collections::HashSet::new();

        // 1. Scan Disk -> Upsert Ledger
        for entry in WalkDir::new(&self.root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                // Relativize path
                if let Ok(rel_path) = path.strip_prefix(&self.root) {
                    let path_str = rel_path.to_string_lossy().to_string(); // Owned string
                    
                    on_disk_paths.insert(path_str.clone());

                    // Ensure DocId exists
                    let doc_id = if let Some(id) = ledger.get_docid(&path_str)? {
                        id
                    } else {
                        let id = ledger.create_docid(&path_str)?;
                        count += 1;
                        id
                    };

                    // Bind Inode (Vital for Rename detection)
                    if let Ok(Some(inode)) = self.get_inode(&path_str) {
                         // We always update the inode mapping to the latest
                         let _ = ledger.bind_inode(&inode, doc_id);
                    }
                }
            }
        }

        // 2. Scan Ledger -> Remove Ghosts
        // Optimization: In a real system, we might query the DB for all paths first.
        let known_docs = ledger.list_docs()?; // Returns (DocId, String)
        let mut removed_count = 0;
        
        for (_id, path) in known_docs {
             // Normalized path check (Windows backslashes vs internal forward slashes?)
             // Ledger paths should match what we insert.
             // If on_disk_paths does not contain it, it's a ghost.
             
             // Check if it exists on disk physically? Or just trust the WalkDir we just did.
             // Trusting WalkDir is faster.
             if !on_disk_paths.contains(&path) {
                 // Double check existence to be safe (race condition?)
                 // If we just walked, it should be accurate.
                 
                 // Remove from ledger
                 match ledger.delete_doc(&path) {
                     Ok(_) => {
                         removed_count += 1; 
                         // tracing::info!("Removed ghost doc: {}", path); // No tracing here, just println in main
                     },
                     Err(_) => {}
                 }
             }
        }
        
        if removed_count > 0 {
             // println!("Cleaned up {} ghost documents.", removed_count);
             // We can return total changes or just additions.
        }

        Ok(count)
    }
}
