// crates/core/src/ledger/node_meta/update.rs
//! # Node 元数据更新操作

use super::split_path;
use crate::ledger::schema::{NODEID_TO_META, PATH_TO_NODEID};
use crate::models::{NodeId, NodeMeta};
use crate::utils::path::to_forward_slash;
use anyhow::{Result, anyhow};
use redb::{Database, ReadableTable};

pub fn remove_node_by_path(db: &Database, path: &str) -> Result<()> {
    let normalized = to_forward_slash(path);
    let write_txn = db.begin_write()?;
    {
        let mut n2m = write_txn.open_table(NODEID_TO_META)?;
        let mut p2n = write_txn.open_table(PATH_TO_NODEID)?;
        if let Some(id) = {
            let guard = p2n.get(&*normalized)?;
            guard.map(|g| g.value())
        } {
            p2n.remove(&*normalized)?;
            n2m.remove(id)?;
        }
    }
    write_txn.commit()?;
    Ok(())
}

pub fn rename_path_prefix(db: &Database, old_prefix: &str, new_prefix: &str) -> Result<()> {
    let old_prefix = to_forward_slash(old_prefix)
        .trim_end_matches('/')
        .to_string();
    let new_prefix = to_forward_slash(new_prefix)
        .trim_end_matches('/')
        .to_string();
    if old_prefix.is_empty() || new_prefix.is_empty() {
        return Err(anyhow!("Empty path prefix is not allowed"));
    }
    if old_prefix == new_prefix {
        return Ok(());
    }
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(PATH_TO_NODEID)?;
    let mut updates = Vec::new();

    for item in table.iter()? {
        let (path_guard, id_guard) = item?;
        let path = path_guard.value();
        if path == old_prefix || path.starts_with(&format!("{}/", old_prefix)) {
            let suffix = &path[old_prefix.len()..];
            let new_path = format!("{}{}", new_prefix, suffix);
            updates.push((path.to_string(), new_path, id_guard.value()));
        }
    }

    drop(table);
    drop(read_txn);

    let write_txn = db.begin_write()?;
    {
        let mut n2m = write_txn.open_table(NODEID_TO_META)?;
        let mut p2n = write_txn.open_table(PATH_TO_NODEID)?;
        let mut touched = Vec::new();

        for (old_path, new_path, node_id) in updates {
            p2n.remove(&*old_path)?;
            p2n.insert(&*new_path, node_id)?;
            if let Some(meta_bytes) = {
                let guard = n2m.get(node_id)?;
                guard.map(|g| g.value().to_vec())
            } {
                let mut meta: NodeMeta = bincode::deserialize(&meta_bytes)?;
                meta.path = new_path.clone();
                meta.name = split_path(&new_path).1.to_string();
                let bytes = bincode::serialize(&meta)?;
                n2m.insert(node_id, bytes.as_slice())?;
                touched.push(node_id);
            }
        }

        for node_id in touched {
            if let Some(meta_bytes) = {
                let guard = n2m.get(node_id)?;
                guard.map(|g| g.value().to_vec())
            } {
                let mut meta: NodeMeta = bincode::deserialize(&meta_bytes)?;
                if meta.path == new_prefix || meta.path == old_prefix {
                    let parent_path = split_path(&meta.path).0;
                    meta.parent_id = if parent_path.is_empty() {
                        None
                    } else {
                        p2n.get(parent_path)?.map(|v| NodeId::from_u128(v.value()))
                    };
                }
                let bytes = bincode::serialize(&meta)?;
                n2m.insert(node_id, bytes.as_slice())?;
            }
        }
    }
    write_txn.commit()?;
    Ok(())
}

pub fn delete_path_prefix(db: &Database, prefix: &str) -> Result<usize> {
    let prefix = to_forward_slash(prefix);
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(PATH_TO_NODEID)?;
    let mut to_delete = Vec::new();

    for item in table.iter()? {
        let (path_guard, id_guard) = item?;
        let path = path_guard.value();
        if path == prefix || path.starts_with(&format!("{}/", prefix)) {
            to_delete.push((path.to_string(), id_guard.value()));
        }
    }

    drop(table);
    drop(read_txn);

    let count = to_delete.len();
    let write_txn = db.begin_write()?;
    {
        let mut n2m = write_txn.open_table(NODEID_TO_META)?;
        let mut p2n = write_txn.open_table(PATH_TO_NODEID)?;
        for (path, node_id) in to_delete {
            p2n.remove(&*path)?;
            n2m.remove(node_id)?;
        }
    }
    write_txn.commit()?;
    Ok(count)
}
