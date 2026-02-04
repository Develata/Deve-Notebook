// crates/core/src/ledger/node_check.rs
//! # Node 一致性检查 (Node Consistency Check)
//!
//! 校验 Doc 表与 Node 表的路径一致性。

use crate::ledger::metadata;
use crate::ledger::node_meta;
use crate::models::{DocId, NodeId};
use anyhow::Result;
use redb::Database;
use std::collections::{BTreeMap, BTreeSet};
use tracing::info;

#[derive(Debug, Clone)]
pub struct NodeConsistencyReport {
    pub missing_nodes: Vec<(DocId, String)>,
    pub orphan_nodes: Vec<(NodeId, String)>,
}

impl NodeConsistencyReport {
    pub fn is_clean(&self) -> bool {
        self.missing_nodes.is_empty() && self.orphan_nodes.is_empty()
    }
}

pub fn check_node_consistency(db: &Database) -> Result<NodeConsistencyReport> {
    let docs = metadata::list_docs(db)?;
    let nodes = node_meta::list_nodes(db)?;

    let mut doc_paths = BTreeSet::new();
    for (_, path) in &docs {
        doc_paths.insert(path.as_str().to_string());
    }

    let mut node_paths = BTreeMap::new();
    for (node_id, meta) in nodes {
        node_paths.insert(meta.path.clone(), (node_id, meta));
    }

    let mut missing_nodes = Vec::new();
    for (doc_id, path) in docs {
        if !node_paths.contains_key(&path) {
            missing_nodes.push((doc_id, path));
        }
    }

    let mut orphan_nodes = Vec::new();
    for (path, (node_id, meta)) in node_paths {
        if meta.doc_id.is_some() && !doc_paths.contains(&path) {
            orphan_nodes.push((node_id, path));
        }
    }

    Ok(NodeConsistencyReport {
        missing_nodes,
        orphan_nodes,
    })
}

pub fn repair_missing_nodes(db: &Database) -> Result<NodeConsistencyReport> {
    let report = check_node_consistency(db)?;
    if report.missing_nodes.is_empty() {
        return Ok(report);
    }

    for (doc_id, path) in &report.missing_nodes {
        node_meta::ensure_file_node(db, path, *doc_id)?;
    }

    let repaired = check_node_consistency(db)?;
    if report.missing_nodes.len() != repaired.missing_nodes.len() {
        info!(
            "Node repair applied: missing {} -> {}",
            report.missing_nodes.len(),
            repaired.missing_nodes.len()
        );
    }
    Ok(repaired)
}
