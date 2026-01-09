use anyhow::Result;
use redb::{Database, TableDefinition, ReadableTable};
use std::path::Path;
use crate::models::{DocId, LedgerEntry, FileNodeId};

// Table Definitions
// DocId (u128) -> Path String
const DOCID_TO_PATH: TableDefinition<u128, &str> = TableDefinition::new("docid_to_path");
// Path String -> DocId (u128)
const PATH_TO_DOCID: TableDefinition<&str, u128> = TableDefinition::new("path_to_docid");
// FileNodeId (u128) -> DocId (u128) - For Rename Detection
const INODE_TO_DOCID: TableDefinition<u128, u128> = TableDefinition::new("inode_to_docid");
// Sequence (u64) -> LedgerEntry (Bytes)
const LEDGER_OPS: TableDefinition<u64, &[u8]> = TableDefinition::new("ledger_ops");
// DocId (u128) -> Vec<u64> (Sequence Numbers) - Secondary Index
// Simpler: Just scan LEDGER_OPS? No, that's O(N). We need an index.
// DocId + Seq -> ()?
// Let's use a MultiMap equivalent? Redb supports Multimap? 
// Redb 2.0 has MultimapTableDefinition.
use redb::MultimapTableDefinition;
const DOC_OPS: MultimapTableDefinition<u128, u64> = MultimapTableDefinition::new("doc_ops");

pub struct Ledger {
    db: Database,
}

impl Ledger {
    pub fn init(path: impl AsRef<Path>) -> Result<Self> {
        let db = Database::create(path)?;
        
        // Initialize tables
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(DOCID_TO_PATH)?;
            let _ = write_txn.open_table(PATH_TO_DOCID)?;
            let _ = write_txn.open_table(INODE_TO_DOCID)?;
            let _ = write_txn.open_table(LEDGER_OPS)?;
            let _ = write_txn.open_multimap_table(DOC_OPS)?;
        }
        write_txn.commit()?;

        Ok(Self { db })
    }

    pub fn get_docid(&self, path: &str) -> Result<Option<DocId>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(PATH_TO_DOCID)?;
        if let Some(v) = table.get(path)? {
            Ok(Some(DocId::from_u128(v.value())))
        } else {
            Ok(None)
        }
    }

    pub fn create_docid(&self, path: &str) -> Result<DocId> {
        let id = DocId::new();
        let write_txn = self.db.begin_write()?;
        {
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;
            
            p2d.insert(path, id.as_u128())?;
            d2p.insert(id.as_u128(), path)?;
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
        let write_txn = self.db.begin_write()?;
        {
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;

            // Get ID
            // Get ID and drop the guard immediately
            let id_opt = p2d.get(old_path)?.map(|v| v.value());

            if let Some(id) = id_opt {
                // Remove old path mapping
                p2d.remove(old_path)?;
                // Insert new path mapping
                p2d.insert(new_path, id)?;
                // Update reverse mapping
                d2p.insert(id, new_path)?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn delete_doc(&self, path: &str) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;

            // Get ID
            let id_opt = p2d.get(path)?.map(|v| v.value());

            if let Some(id) = id_opt {
                p2d.remove(path)?;
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
            
            // Scan through all paths (Inefficient for huge DB, but OK for MVP)
            // Ideally use range query if paths were keys, but p2d key is string.
            // Range scan? old_prefix..old_prefix+1? String range scan is possible in redb.
            // But let's just iter for now or use range.
            // p2d.range(old_prefix..)?
            
            for item in p2d.iter()? {
                let (path_guard, id_guard) = item?;
                let path = path_guard.value();
                let id = id_guard.value();
                
                // Check if path is exactly old_prefix (if it was a file, but handled elsewhere)
                // or starts with old_prefix + / or \
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
