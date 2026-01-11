use anyhow::Result;
use redb::{Database, ReadableTable, ReadableMultimapTable};
use std::path::Path;
use crate::models::{DocId, LedgerEntry, FileNodeId};
use crate::utils::path::to_forward_slash;

pub mod schema;
use self::schema::*;

pub struct Ledger {
    pub db: Database,
    pub snapshot_depth: usize,
}

impl Ledger {
    pub fn init(path: impl AsRef<Path>, snapshot_depth: usize) -> Result<Self> {
        let db = Database::create(path)?;
        
        // Initialize tables
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(DOCID_TO_PATH)?;
            let _ = write_txn.open_table(PATH_TO_DOCID)?;
            let _ = write_txn.open_table(INODE_TO_DOCID)?;
            let _ = write_txn.open_table(LEDGER_OPS)?;
            let _ = write_txn.open_multimap_table(DOC_OPS)?;
            let _ = write_txn.open_multimap_table(SNAPSHOT_INDEX)?;
            let _ = write_txn.open_table(SNAPSHOT_DATA)?;
        }
        write_txn.commit()?;

        Ok(Self { db, snapshot_depth })
    }

    /// Save a snapshot for a document.
    pub fn save_snapshot(&self, doc_id: DocId, seq: u64, content: &str) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut index = write_txn.open_multimap_table(SNAPSHOT_INDEX)?;
            let mut data = write_txn.open_table(SNAPSHOT_DATA)?;
            
            // 1. Insert Data
            data.insert(seq, content.as_bytes())?;
            
            // 2. Index
            index.insert(doc_id.as_u128(), seq)?;
        }
        write_txn.commit()?;
        
        // Trigger pruning
        self.prune_snapshots(doc_id)?;
        
        Ok(())
    }

    /// Prune old snapshots if they exceed the configured depth.
    fn prune_snapshots(&self, doc_id: DocId) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        let count = {
            // Phase 1: Collect
            let mut snapshots = Vec::new();
            {
                let index = write_txn.open_multimap_table(SNAPSHOT_INDEX)?;
                let iter = index.get(doc_id.as_u128())?;
                for item in iter {
                    let seq: u64 = item?.value();
                    snapshots.push(seq);
                }
            }
            
            // Phase 2: Prune
            snapshots.sort();
            let total = snapshots.len();
            if total > self.snapshot_depth {
                let to_remove = total - self.snapshot_depth;
                let remove_seqs = &snapshots[0..to_remove];
                
                let mut index = write_txn.open_multimap_table(SNAPSHOT_INDEX)?;
                let mut data = write_txn.open_table(SNAPSHOT_DATA)?;
                
                for &seq in remove_seqs {
                    index.remove(doc_id.as_u128(), seq)?;
                    data.remove(seq)?;
                }
                to_remove
            } else {
                0
            }
        };
        write_txn.commit()?;
        
        Ok(())
    }

    pub fn get_docid(&self, path: &str) -> Result<Option<DocId>> {
        let normalized = to_forward_slash(path);
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(PATH_TO_DOCID)?;
        if let Some(v) = table.get(&*normalized)? {
            Ok(Some(DocId::from_u128(v.value())))
        } else {
            Ok(None)
        }
    }

    pub fn create_docid(&self, path: &str) -> Result<DocId> {
        let normalized = to_forward_slash(path);
        let id = DocId::new();
        let write_txn = self.db.begin_write()?;
        {
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;
            
            p2d.insert(&*normalized, id.as_u128())?;
            d2p.insert(id.as_u128(), &*normalized)?;
        }
        write_txn.commit()?;
        Ok(id)
    }

    pub fn get_path_by_docid(&self, doc_id: DocId) -> Result<Option<String>> {
         let read_txn = self.db.begin_read()?;
         let table = read_txn.open_table(DOCID_TO_PATH)?;
         if let Some(v) = table.get(doc_id.as_u128())? {
             Ok(Some(v.value().to_string()))
         } else {
             Ok(None)
         }
    }

    pub fn get_docid_by_inode(&self, inode: &FileNodeId) -> Result<Option<DocId>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(INODE_TO_DOCID)?;
        if let Some(v) = table.get(inode.id)? {
            Ok(Some(DocId::from_u128(v.value())))
        } else {
            Ok(None)
        }
    }

    pub fn bind_inode(&self, inode: &FileNodeId, doc_id: DocId) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(INODE_TO_DOCID)?;
            table.insert(inode.id, doc_id.as_u128())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn rename_doc(&self, old_path: &str, new_path: &str) -> Result<()> {
        let old_normalized = to_forward_slash(old_path);
        let new_normalized = to_forward_slash(new_path);
        let write_txn = self.db.begin_write()?;
        {
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;

            // Get ID
            let id_opt = p2d.get(&*old_normalized)?.map(|v| v.value());

            if let Some(id) = id_opt {
                // Remove old path mapping
                p2d.remove(&*old_normalized)?;
                // Insert new path mapping
                p2d.insert(&*new_normalized, id)?;
                // Update reverse mapping
                d2p.insert(id, &*new_normalized)?;
            } else {
                return Err(anyhow::anyhow!("Document not found in ledger: {}", old_path));
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn delete_doc(&self, path: &str) -> Result<()> {
        let normalized = to_forward_slash(path);
        let write_txn = self.db.begin_write()?;
        {
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;

            // Get ID
            let id_opt = p2d.get(&*normalized)?.map(|v| v.value());

            if let Some(id) = id_opt {
                p2d.remove(&*normalized)?;
                d2p.remove(id)?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn rename_folder(&self, old_prefix: &str, new_prefix: &str) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;
            
            // 1. Collect all affected paths first (to avoid borrowing issues while writing)
            let mut updates = Vec::new();
            
            for item in p2d.iter()? {
                let (path_guard, id_guard) = item?;
                let path = path_guard.value();
                let id = id_guard.value();
                
                if path == old_prefix || path.starts_with(&format!("{}/", old_prefix)) || path.starts_with(&format!("{}\\", old_prefix)) {
                     // Calculate new path
                     let suffix = &path[old_prefix.len()..];
                     let new_path = format!("{}{}", new_prefix, suffix);
                     updates.push((path.to_string(), new_path, id));
                }
            }
            
            // 2. Apply updates
            for (old, new, id) in updates {
                p2d.remove(&*old)?;
                p2d.insert(&*new, id)?;
                d2p.insert(id, &*new)?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Delete all documents whose paths start with the given prefix (folder deletion).
    pub fn delete_folder(&self, prefix: &str) -> Result<usize> {
        let write_txn = self.db.begin_write()?;
        let count = {
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;
            
            // Collect all docs to delete
            let mut to_delete = Vec::new();
            
            for item in p2d.iter()? {
                let (path_guard, id_guard) = item?;
                let path = path_guard.value();
                let id = id_guard.value();
                
                if path == prefix 
                    || path.starts_with(&format!("{}/", prefix)) 
                    || path.starts_with(&format!("{}\\", prefix)) 
                {
                    to_delete.push((path.to_string(), id));
                }
            }
            
            let count = to_delete.len();
            
            for (path, id) in to_delete {
                p2d.remove(&*path)?;
                d2p.remove(id)?;
            }
            
            count
        };
        write_txn.commit()?;
        Ok(count)
    }

    pub fn append_op(&self, entry: &LedgerEntry) -> Result<u64> {
        let write_txn = self.db.begin_write()?;
        let seq = {
            let mut ops = write_txn.open_table(LEDGER_OPS)?;
            let mut doc_ops = write_txn.open_multimap_table(DOC_OPS)?;
            
            let last_seq = ops.last()?.map(|(k, _)| k.value()).unwrap_or(0u64);
            let new_seq = last_seq + 1;
            let bytes = bincode::serialize(entry)?;
            ops.insert(new_seq, bytes.as_slice())?;
            
            // Index by DocId
            doc_ops.insert(entry.doc_id.as_u128(), new_seq)?;
            
            new_seq
        };
        write_txn.commit()?;
        Ok(seq)
    }

    pub fn get_ops(&self, doc_id: DocId) -> Result<Vec<(u64, LedgerEntry)>> {
        let read_txn = self.db.begin_read()?;
        let ops_table = read_txn.open_table(LEDGER_OPS)?;
        let doc_ops_table = read_txn.open_multimap_table(DOC_OPS)?;
        
        let mut entries = Vec::new();
        let seqs = doc_ops_table.get(doc_id.as_u128())?;
        
        for seq in seqs {
            let seq_val = seq?.value();
            if let Some(bytes) = ops_table.get(seq_val)? {
                 let entry: LedgerEntry = bincode::deserialize(bytes.value())?;
                 entries.push((seq_val, entry));
            }
        }
        
        // Sort by sequence number
        entries.sort_by_key(|k| k.0);
        
        Ok(entries)
    }

    pub fn list_docs(&self) -> Result<Vec<(DocId, String)>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(DOCID_TO_PATH)?;
        let mut docs = Vec::new();
        for item in table.iter()? {
            let (id, path) = item?;
            docs.push((DocId::from_u128(id.value()), path.value().to_string()));
        }
        Ok(docs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use tempfile::NamedTempFile;

    #[test]
    fn test_snapshot_pruning() -> Result<()> {
        let tmp_file = NamedTempFile::new()?;
        let db_path = tmp_file.path();
        
        // Init ledger with snapshot_depth = 2
        let ledger = Ledger::init(db_path, 2)?;
        let doc_id = DocId::new();
        
        // Save 3 snapshots
        ledger.save_snapshot(doc_id, 1, "Snap 1")?;
        ledger.save_snapshot(doc_id, 2, "Snap 2")?;
        ledger.save_snapshot(doc_id, 3, "Snap 3")?; // This should prune seq 1
        
        // Verify pruning
        let read_txn = ledger.db.begin_read()?;
        let index = read_txn.open_multimap_table(SNAPSHOT_INDEX)?;
        let data = read_txn.open_table(SNAPSHOT_DATA)?;
        
        // Check Index
        let mut seqs = Vec::new();
        for item in index.get(doc_id.as_u128())? {
            seqs.push(item?.value());
        }
        seqs.sort();
        
        assert_eq!(seqs, vec![2, 3], "Snapshot index should only contain 2 and 3");
        
        // Check Data
        assert!(data.get(1)?.is_none(), "Snapshot 1 data should be removed");
        assert!(data.get(2)?.is_some(), "Snapshot 2 data should exist");
        assert!(data.get(3)?.is_some(), "Snapshot 3 data should exist");
        
        Ok(())
    }
}
