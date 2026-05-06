// crates\core\src\ledger
//! plan_ref:
//!   - 06_repository#tree-projection-contract
//!   - 04_storage#projection-contract
//!   - 04_storage#internal-path-normalization
//!
//! # 元数据映射模块 (Metadata Mapping)
//!
//! 管理 Path/DocId/Inode 之间的映射关系。
//! 所有映射仅存储在 local.redb 中。

#[path = "metadata_tree.rs"]
mod tree;

use crate::ledger::node_meta;
use crate::ledger::schema::*;
use crate::models::DocId;
use crate::utils::path::to_forward_slash;
use anyhow::Result;
use redb::{Database, ReadableTable, WriteTransaction};
pub use tree::{delete_folder, rename_folder};
pub(crate) use tree::{delete_folder_in_txn, rename_folder_in_txn};

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

pub fn set_doc_path(db: &Database, doc_id: DocId, path: &str) -> Result<()> {
    let write_txn = db.begin_write()?;
    set_doc_path_in_txn(&write_txn, doc_id, path)?;
    write_txn.commit()?;
    Ok(())
}

pub(crate) fn set_doc_path_in_txn(
    write_txn: &WriteTransaction,
    doc_id: DocId,
    path: &str,
) -> Result<()> {
    let normalized = to_forward_slash(path);
    {
        let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
        let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;

        if let Some(existing) = p2d.get(&*normalized)?.map(|v| v.value())
            && existing != doc_id.as_u128()
        {
            return Err(anyhow::anyhow!(
                "Path already bound to another document: {}",
                normalized
            ));
        }

        if let Some(old_path) = d2p.get(doc_id.as_u128())?.map(|v| v.value().to_string())
            && old_path != normalized
        {
            p2d.remove(&*old_path)?;
        }

        p2d.insert(&*normalized, doc_id.as_u128())?;
        d2p.insert(doc_id.as_u128(), &*normalized)?;
    }
    node_meta::ensure_file_node_in_txn(write_txn, &normalized, doc_id)?;
    Ok(())
}

pub fn rename_doc(db: &Database, old_path: &str, new_path: &str) -> Result<()> {
    let write_txn = db.begin_write()?;
    rename_doc_in_txn(&write_txn, old_path, new_path)?;
    write_txn.commit()?;
    Ok(())
}

pub(crate) fn rename_doc_in_txn(
    write_txn: &WriteTransaction,
    old_path: &str,
    new_path: &str,
) -> Result<()> {
    let old_normalized = to_forward_slash(old_path);
    let new_normalized = to_forward_slash(new_path);
    {
        let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
        let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;

        let id_opt = p2d.get(&*old_normalized)?.map(|v| v.value());

        if let Some(id) = id_opt {
            if let Some(existing) = p2d.get(&*new_normalized)?.map(|v| v.value())
                && existing != id
            {
                return Err(anyhow::anyhow!(
                    "Path already bound to another document: {}",
                    new_normalized
                ));
            }
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
    node_meta::rename_path_prefix_in_txn(write_txn, &old_normalized, &new_normalized)?;
    Ok(())
}

pub fn delete_doc(db: &Database, path: &str) -> Result<()> {
    let write_txn = db.begin_write()?;
    delete_doc_in_txn(&write_txn, path)?;
    write_txn.commit()?;
    Ok(())
}

pub(crate) fn delete_doc_in_txn(write_txn: &WriteTransaction, path: &str) -> Result<()> {
    let normalized = to_forward_slash(path);
    {
        let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
        let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;

        let id_opt = p2d.get(&*normalized)?.map(|v| v.value());

        if let Some(id) = id_opt {
            p2d.remove(&*normalized)?;
            d2p.remove(id)?;
        }
    }
    node_meta::remove_node_by_path_in_txn(write_txn, &normalized)?;
    Ok(())
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
