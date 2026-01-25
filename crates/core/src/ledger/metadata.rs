// crates\core\src\ledger
//! # 元数据映射模块 (Metadata Mapping)
//!
//! 管理 Path/DocId/Inode 之间的映射关系。
//! 所有映射仅存储在 local.redb 中。

use crate::ledger::schema::*;
use crate::models::{DocId, FileNodeId};
use crate::utils::path::to_forward_slash;
use anyhow::Result;
use redb::{Database, ReadableTable};

pub fn get_docid(db: &Database, path: &str) -> Result<Option<DocId>> {
    let normalized = to_forward_slash(path);
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(PATH_TO_DOCID)?;
    if let Some(v) = table.get(&*normalized)? {
        Ok(Some(DocId::from_u128(v.value())))
    } else {
        Ok(None)
    }
}

pub fn create_docid(db: &Database, path: &str) -> Result<DocId> {
    let normalized = to_forward_slash(path);
    let id = DocId::new();
    let write_txn = db.begin_write()?;
    {
        let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
        let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;

        p2d.insert(&*normalized, id.as_u128())?;
        d2p.insert(id.as_u128(), &*normalized)?;
    }
    write_txn.commit()?;
    Ok(id)
}

pub fn get_path_by_docid(db: &Database, doc_id: DocId) -> Result<Option<String>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(DOCID_TO_PATH)?;
    if let Some(v) = table.get(doc_id.as_u128())? {
        Ok(Some(v.value().to_string()))
    } else {
        Ok(None)
    }
}

pub fn get_docid_by_inode(db: &Database, inode: &FileNodeId) -> Result<Option<DocId>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(INODE_TO_DOCID)?;
    if let Some(v) = table.get(inode.id)? {
        Ok(Some(DocId::from_u128(v.value())))
    } else {
        Ok(None)
    }
}

pub fn bind_inode(db: &Database, inode: &FileNodeId, doc_id: DocId) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(INODE_TO_DOCID)?;
        table.insert(inode.id, doc_id.as_u128())?;
    }
    write_txn.commit()?;
    Ok(())
}

pub fn rename_doc(db: &Database, old_path: &str, new_path: &str) -> Result<()> {
    let old_normalized = to_forward_slash(old_path);
    let new_normalized = to_forward_slash(new_path);
    let write_txn = db.begin_write()?;
    {
        let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
        let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;

        let id_opt = p2d.get(&*old_normalized)?.map(|v| v.value());

        if let Some(id) = id_opt {
            p2d.remove(&*old_normalized)?;
            p2d.insert(&*new_normalized, id)?;
            d2p.insert(id, &*new_normalized)?;
        } else {
            return Err(anyhow::anyhow!(
                "Document not found in ledger: {}",
                old_path
            ));
        }
    }
    write_txn.commit()?;
    Ok(())
}

pub fn delete_doc(db: &Database, path: &str) -> Result<()> {
    let normalized = to_forward_slash(path);
    let write_txn = db.begin_write()?;
    {
        let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
        let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;

        let id_opt = p2d.get(&*normalized)?.map(|v| v.value());

        if let Some(id) = id_opt {
            p2d.remove(&*normalized)?;
            d2p.remove(id)?;
        }
    }
    write_txn.commit()?;
    Ok(())
}

/// 重命名文件夹
///
/// TODO: Optimization - splitting into batches locally is difficult due to Atomicity requirements.
/// Consider using a Journal/WAL if performance issues arise with >10k files.
pub fn rename_folder(db: &Database, old_prefix: &str, new_prefix: &str) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
        let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;

        let mut updates = Vec::new();

        for item in p2d.iter()? {
            let (path_guard, id_guard) = item?;
            let path = path_guard.value();
            let id = id_guard.value();

            if path == old_prefix
                || path.starts_with(&format!("{}/", old_prefix))
                || path.starts_with(&format!("{}\\", old_prefix))
            {
                let suffix = &path[old_prefix.len()..];
                let new_path = format!("{}{}", new_prefix, suffix);
                updates.push((path.to_string(), new_path, id));
            }
        }

        for (old, new, id) in updates {
            p2d.remove(&*old)?;
            p2d.insert(&*new, id)?;
            d2p.insert(id, &*new)?;
        }
    }
    write_txn.commit()?;
    Ok(())
}

pub fn delete_folder(db: &Database, prefix: &str) -> Result<usize> {
    let write_txn = db.begin_write()?;
    let count = {
        let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
        let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;

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

pub fn list_docs(db: &Database) -> Result<Vec<(DocId, String)>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(DOCID_TO_PATH)?;
    let mut docs = Vec::new();
    for item in table.iter()? {
        let (id, path) = item?;
        docs.push((DocId::from_u128(id.value()), path.value().to_string()));
    }
    tracing::info!("Listed {} docs from DB", docs.len());
    Ok(docs)
}
