//! plan_ref:
//!   - 17_tech_stack#graph-visualization
//!
//! # Read-only knowledge graph projection
//!
//! This module derives graph nodes and edges from already-selected repo
//! documents. It is a projection helper only: it does not read or write ledger,
//! metadata tables, workspace files, search indexes, or source-control state.

use self::links::{extract_link_candidates, resolve_link_target};
use self::path::{normalize_doc_path, title_from_path};
use crate::models::DocId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod links;
mod path;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphDocument {
    pub doc_id: DocId,
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphProjection {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub unresolved_links: Vec<UnresolvedGraphLink>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNode {
    pub doc_id: DocId,
    pub path: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from_doc_id: DocId,
    pub to_doc_id: DocId,
    pub link_text: String,
    pub target_path: String,
    pub kind: GraphLinkKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnresolvedGraphLink {
    pub from_doc_id: DocId,
    pub source_path: String,
    pub link_text: String,
    pub target_path: String,
    pub kind: GraphLinkKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphLinkKind {
    Wiki,
    Markdown,
}

pub fn project_documents(docs: &[GraphDocument]) -> GraphProjection {
    let mut nodes = Vec::with_capacity(docs.len());
    let mut by_path = HashMap::with_capacity(docs.len());

    for doc in docs {
        let path = normalize_doc_path(&doc.path);
        by_path.insert(path.clone(), doc.doc_id);
        nodes.push(GraphNode {
            doc_id: doc.doc_id,
            title: title_from_path(&path),
            path,
        });
    }

    nodes.sort_by(|a, b| a.path.cmp(&b.path));

    let mut edges = Vec::new();
    let mut unresolved_links = Vec::new();
    for doc in docs {
        let source_path = normalize_doc_path(&doc.path);
        for candidate in extract_link_candidates(&doc.content) {
            let target_path = resolve_link_target(&source_path, &candidate);
            if let Some(to_doc_id) = by_path.get(&target_path).copied() {
                edges.push(GraphEdge {
                    from_doc_id: doc.doc_id,
                    to_doc_id,
                    link_text: candidate.text,
                    target_path,
                    kind: candidate.kind,
                });
            } else {
                unresolved_links.push(UnresolvedGraphLink {
                    from_doc_id: doc.doc_id,
                    source_path: source_path.clone(),
                    link_text: candidate.text,
                    target_path,
                    kind: candidate.kind,
                });
            }
        }
    }

    edges.sort_by(|a, b| {
        a.from_doc_id
            .as_u128()
            .cmp(&b.from_doc_id.as_u128())
            .then_with(|| a.target_path.cmp(&b.target_path))
            .then_with(|| a.link_text.cmp(&b.link_text))
    });
    unresolved_links.sort_by(|a, b| {
        a.source_path
            .cmp(&b.source_path)
            .then_with(|| a.target_path.cmp(&b.target_path))
            .then_with(|| a.link_text.cmp(&b.link_text))
    });

    GraphProjection {
        nodes,
        edges,
        unresolved_links,
    }
}
