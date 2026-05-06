//! plan_ref:
//!   - 06_repository#tree-projection-contract
//!   - 04_storage#projection-contract
//!   - 04_storage#internal-path-normalization

use crate::ledger::node_meta;
use crate::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use crate::utils::path::to_forward_slash;
use anyhow::Result;
use redb::{Database, ReadableTable, WriteTransaction};

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

fn is_under(path: &str, prefix: &str) -> bool {
    path == prefix || path.starts_with(&format!("{}/", prefix))
}
