// crates/core/src/ledger/node_meta/core.rs
//! # Node 元数据核心操作

use super::split_path;
use crate::ledger::schema::{NODEID_TO_META, PATH_TO_NODEID};
use crate::models::{DocId, NodeId, NodeKind, NodeMeta};
use crate::utils::path::to_forward_slash;
use anyhow::{Result, anyhow};
use redb::Database;

pub fn get_node_id(db: &Database, path: &str) -> Result<Option<NodeId>> {
    let normalized = to_forward_slash(path);
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(PATH_TO_NODEID)?;
    if let Some(v) = table.get(&*normalized)? {
        Ok(Some(NodeId::from_u128(v.value())))
    } else {
        Ok(None)
    }
}

pub fn get_node_meta(db: &Database, node_id: NodeId) -> Result<Option<NodeMeta>> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(NODEID_TO_META)?;
    if let Some(v) = table.get(node_id.as_u128())? {
        let meta: NodeMeta = bincode::deserialize(v.value())?;
        Ok(Some(meta))
    } else {
        Ok(None)
    }
}

pub fn upsert_node(db: &Database, node_id: NodeId, meta: &NodeMeta) -> Result<()> {
    let write_txn = db.begin_write()?;
    {
        let mut n2m = write_txn.open_table(NODEID_TO_META)?;
        let mut p2n = write_txn.open_table(PATH_TO_NODEID)?;
        let bytes = bincode::serialize(meta)?;
        n2m.insert(node_id.as_u128(), bytes.as_slice())?;
        p2n.insert(&*meta.path, node_id.as_u128())?;
    }
    write_txn.commit()?;
    Ok(())
}

pub fn ensure_dir_chain(db: &Database, path: &str) -> Result<Option<NodeId>> {
    let normalized = to_forward_slash(path).trim_end_matches('/').to_string();
    if normalized.is_empty() {
        return Ok(None);
    }

    let parts: Vec<&str> = normalized.split('/').collect();
    let mut current = String::new();
    let mut parent_id: Option<NodeId> = None;
    let mut last_id = None;

    for part in parts {
        if part.is_empty() {
            return Err(anyhow!("Invalid path segment: {}", normalized));
        }
        if !current.is_empty() {
            current.push('/');
        }
        current.push_str(part);

        if let Some(existing) = get_node_id(db, &current)? {
            let meta = get_node_meta(db, existing)?
                .ok_or_else(|| anyhow!("Node meta missing: {}", current))?;
            if meta.kind != NodeKind::Dir {
                return Err(anyhow!("Path is not a directory: {}", current));
            }
            last_id = Some(existing);
            parent_id = Some(existing);
            continue;
        }

        let node_id = NodeId::new();
        let meta = NodeMeta {
            kind: NodeKind::Dir,
            name: part.to_string(),
            parent_id,
            path: current.clone(),
            doc_id: None,
        };
        upsert_node(db, node_id, &meta)?;
        last_id = Some(node_id);
        parent_id = Some(node_id);
    }

    Ok(last_id)
}

pub fn ensure_file_node(db: &Database, path: &str, doc_id: DocId) -> Result<NodeId> {
    let normalized = to_forward_slash(path);
    if normalized.ends_with('/') {
        return Err(anyhow!("File path must not end with '/': {}", normalized));
    }
    if let Some(existing) = get_node_id(db, &normalized)? {
        let meta = get_node_meta(db, existing)?
            .ok_or_else(|| anyhow!("Node meta missing: {}", normalized))?;
        if meta.kind == NodeKind::Dir {
            return Err(anyhow!("Path is a directory: {}", normalized));
        }
        let expected = NodeId::from_doc_id(doc_id);
        if existing != expected {
            return Err(anyhow!("NodeId mismatch for file: {}", normalized));
        }
        return Ok(existing);
    }
    let (parent_path, name) = split_path(&normalized);
    let parent_id = ensure_dir_chain(db, parent_path)?;
    let node_id = NodeId::from_doc_id(doc_id);

    let meta = NodeMeta {
        kind: NodeKind::File,
        name: name.to_string(),
        parent_id,
        path: normalized,
        doc_id: Some(doc_id),
    };
    upsert_node(db, node_id, &meta)?;
    Ok(node_id)
}

pub fn create_dir_node(db: &Database, path: &str) -> Result<NodeId> {
    let normalized = to_forward_slash(path).trim_end_matches('/').to_string();
    if normalized.is_empty() {
        return Err(anyhow!("Empty path is not a valid directory"));
    }

    if let Some(existing) = get_node_id(db, &normalized)? {
        let meta = get_node_meta(db, existing)?
            .ok_or_else(|| anyhow!("Node meta missing: {}", normalized))?;
        if meta.kind != NodeKind::Dir {
            return Err(anyhow!("Path is not a directory: {}", normalized));
        }
        return Ok(existing);
    }

    let (parent_path, name) = split_path(&normalized);
    let parent_id = ensure_dir_chain(db, parent_path)?;
    let node_id = NodeId::new();
    let meta = NodeMeta {
        kind: NodeKind::Dir,
        name: name.to_string(),
        parent_id,
        path: normalized,
        doc_id: None,
    };
    upsert_node(db, node_id, &meta)?;
    Ok(node_id)
}
