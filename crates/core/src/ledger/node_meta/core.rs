// crates/core/src/ledger/node_meta/core.rs
//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 03_storage#projection-contract
//!   - 03_storage#internal-path-normalization
//!
//! # Node 元数据核心操作

use super::lookup::{
    get_node_id, get_node_id_in_txn, get_node_meta, get_node_meta_in_txn, path_doc_in_txn,
};
use super::split_path;
use crate::ledger::schema::{NODEID_TO_META, PATH_TO_NODEID};
use crate::models::{DocId, NodeId, NodeKind, NodeMeta};
use crate::utils::path::to_forward_slash;
use anyhow::{Result, anyhow};
use redb::{Database, WriteTransaction};

pub fn upsert_node(db: &Database, node_id: NodeId, meta: &NodeMeta) -> Result<()> {
    let write_txn = db.begin_write()?;
    upsert_node_in_txn(&write_txn, node_id, meta)?;
    write_txn.commit()?;
    Ok(())
}

pub(crate) fn upsert_node_in_txn(
    write_txn: &WriteTransaction,
    node_id: NodeId,
    meta: &NodeMeta,
) -> Result<()> {
    if let Some(existing_node) = get_node_id_in_txn(write_txn, &meta.path)?
        && existing_node != node_id
    {
        return Err(anyhow!("Path already bound to another node: {}", meta.path));
    }
    if let Some(doc_id) = path_doc_in_txn(write_txn, &meta.path)?
        && Some(doc_id) != meta.doc_id
    {
        return Err(anyhow!(
            "Path already bound to another document: {}",
            meta.path
        ));
    }
    if let Some(existing_meta) = get_node_meta_in_txn(write_txn, node_id)?
        && existing_meta.path != meta.path
    {
        write_txn
            .open_table(PATH_TO_NODEID)?
            .remove(&*existing_meta.path)?;
    }
    {
        let mut n2m = write_txn.open_table(NODEID_TO_META)?;
        let mut p2n = write_txn.open_table(PATH_TO_NODEID)?;
        let bytes = bincode::serialize(meta)?;
        n2m.insert(node_id.as_u128(), bytes.as_slice())?;
        p2n.insert(&*meta.path, node_id.as_u128())?;
    }
    Ok(())
}

pub fn ensure_dir_chain(db: &Database, path: &str) -> Result<Option<NodeId>> {
    let write_txn = db.begin_write()?;
    let node_id = ensure_dir_chain_in_txn(&write_txn, path)?;
    write_txn.commit()?;
    Ok(node_id)
}

pub(crate) fn ensure_dir_chain_in_txn(
    write_txn: &WriteTransaction,
    path: &str,
) -> Result<Option<NodeId>> {
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

        if let Some(existing) = get_node_id_in_txn(write_txn, &current)? {
            let meta = get_node_meta_in_txn(write_txn, existing)?
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
        upsert_node_in_txn(write_txn, node_id, &meta)?;
        last_id = Some(node_id);
        parent_id = Some(node_id);
    }

    Ok(last_id)
}

pub fn ensure_file_node(db: &Database, path: &str, doc_id: DocId) -> Result<NodeId> {
    let write_txn = db.begin_write()?;
    let node_id = ensure_file_node_in_txn(&write_txn, path, doc_id)?;
    write_txn.commit()?;
    Ok(node_id)
}

pub(crate) fn ensure_file_node_in_txn(
    write_txn: &WriteTransaction,
    path: &str,
    doc_id: DocId,
) -> Result<NodeId> {
    let normalized = to_forward_slash(path);
    if normalized.ends_with('/') {
        return Err(anyhow!("File path must not end with '/': {}", normalized));
    }
    if let Some(existing) = get_node_id_in_txn(write_txn, &normalized)? {
        let meta = get_node_meta_in_txn(write_txn, existing)?
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
    if let Some(existing_doc) = path_doc_in_txn(write_txn, &normalized)?
        && existing_doc != doc_id
    {
        return Err(anyhow!(
            "Path already bound to another document: {}",
            normalized
        ));
    }
    let (parent_path, name) = split_path(&normalized);
    let parent_id = ensure_dir_chain_in_txn(write_txn, parent_path)?;
    let node_id = NodeId::from_doc_id(doc_id);

    let meta = NodeMeta {
        kind: NodeKind::File,
        name: name.to_string(),
        parent_id,
        path: normalized,
        doc_id: Some(doc_id),
    };
    upsert_node_in_txn(write_txn, node_id, &meta)?;
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
