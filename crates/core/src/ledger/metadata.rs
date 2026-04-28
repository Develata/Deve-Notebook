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

use crate::ledger::node_meta;
use crate::ledger::schema::*;
use crate::models::DocId;
use crate::utils::path::to_forward_slash;
use anyhow::Result;
use redb::{Database, ReadableTable, WriteTransaction};

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

/// 重命名文件夹
///
/// 保持单事务更新以维持 repair 路径的原子性。
/// 当前 768 MB VPS 目标和 repair-only 调用频率下，不引入额外 WAL/分批复杂度。
pub fn rename_folder(db: &Database, old_prefix: &str, new_prefix: &str) -> Result<()> {
    let write_txn = db.begin_write()?;
    rename_folder_in_txn(&write_txn, old_prefix, new_prefix)?;
    write_txn.commit()?;
    Ok(())
}

pub(crate) fn rename_folder_in_txn(
    write_txn: &WriteTransaction,
    old_prefix: &str,
    new_prefix: &str,
) -> Result<()> {
    let old_prefix = to_forward_slash(old_prefix)
        .trim_end_matches('/')
        .to_string();
    let new_prefix = to_forward_slash(new_prefix)
        .trim_end_matches('/')
        .to_string();
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
            } else if path == new_prefix || path.starts_with(&format!("{}/", new_prefix)) {
                return Err(anyhow::anyhow!(
                    "Path already bound under target folder: {}",
                    path
                ));
            }
        }

        for (old, new, id) in updates {
            if let Some(existing) = p2d.get(&*new)?.map(|v| v.value())
                && existing != id
                && !is_under(&new, &old_prefix)
            {
                return Err(anyhow::anyhow!(
                    "Path already bound to another document: {}",
                    new
                ));
            }
            p2d.remove(&*old)?;
            p2d.insert(&*new, id)?;
            d2p.insert(id, &*new)?;
        }
    }
    node_meta::rename_path_prefix_in_txn(write_txn, &old_prefix, &new_prefix)?;
    Ok(())
}

pub fn delete_folder(db: &Database, prefix: &str) -> Result<usize> {
    let write_txn = db.begin_write()?;
    let count = delete_folder_in_txn(&write_txn, prefix)?;
    write_txn.commit()?;
    Ok(count)
}

pub(crate) fn delete_folder_in_txn(write_txn: &WriteTransaction, prefix: &str) -> Result<usize> {
    let prefix = to_forward_slash(prefix);
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
    let node_count = node_meta::delete_path_prefix_in_txn(write_txn, &prefix)?;
    tracing::debug!("NodeMeta deleted: {}", node_count);
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

fn is_under(path: &str, prefix: &str) -> bool {
    path == prefix || path.starts_with(&format!("{}/", prefix))
}
