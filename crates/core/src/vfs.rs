use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::ledger::Ledger;
use crate::models::FileNodeId;
use file_id::FileId;

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
    pub fn scan(&self, ledger: &Ledger) -> Result<usize> {
        let mut count = 0;
        for entry in WalkDir::new(&self.root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                // Relativize path
                if let Ok(rel_path) = path.strip_prefix(&self.root) {
                    let path_str = rel_path.to_string_lossy();
                    
                    // 1. Ensure DocId exists
                    let doc_id = if let Some(id) = ledger.get_docid(&path_str)? {
                        id
                    } else {
                        let id = ledger.create_docid(&path_str)?;
                        count += 1;
                        id
                    };

                    // 2. Bind Inode (Vital for Rename detection)
                    if let Ok(Some(inode)) = self.get_inode(&path_str) {
                         // We always update the inode mapping to the latest
                         let _ = ledger.bind_inode(&inode, doc_id);
                    }
                }
            }
        }
        Ok(count)
    }
}
