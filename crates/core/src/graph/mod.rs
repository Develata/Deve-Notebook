//! plan_ref:
//!   - 14_tech_stack#graph-visualization
//!
//! # Read-only knowledge graph projection
//!
//! This module derives graph nodes and edges from already-selected repo
//! documents. It is a projection helper only: it does not read or write ledger,
//! metadata tables, workspace files, search indexes, or source-control state.

use crate::models::DocId;
use crate::utils::path::to_forward_slash;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct LinkCandidate {
    text: String,
    target: String,
    kind: GraphLinkKind,
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

fn normalize_doc_path(path: &str) -> String {
    to_forward_slash(path).trim_start_matches("./").to_string()
}

fn title_from_path(path: &str) -> String {
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .strip_suffix(".md")
        .unwrap_or_else(|| path.rsplit('/').next().unwrap_or(path))
        .to_string()
}

fn extract_link_candidates(content: &str) -> Vec<LinkCandidate> {
    let mut links = Vec::new();
    extract_wiki_links(content, &mut links);
    extract_markdown_links(content, &mut links);
    links
}

fn extract_wiki_links(content: &str, links: &mut Vec<LinkCandidate>) {
    let mut rest = content;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else {
            break;
        };
        let raw = rest[..end].trim();
        if !raw.is_empty() {
            let (target, text) = split_wiki_link(raw);
            links.push(LinkCandidate {
                text,
                target,
                kind: GraphLinkKind::Wiki,
            });
        }
        rest = &rest[end + 2..];
    }
}

fn split_wiki_link(raw: &str) -> (String, String) {
    let mut parts = raw.splitn(2, '|');
    let target = parts.next().unwrap_or_default().trim().to_string();
    let text = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(target.as_str())
        .to_string();
    (target, text)
}

fn extract_markdown_links(content: &str, links: &mut Vec<LinkCandidate>) {
    let bytes = content.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        let Some(label_start) = content[idx..].find('[').map(|offset| idx + offset) else {
            break;
        };
        if content[label_start..].starts_with("[[") {
            idx = label_start + 2;
            continue;
        }
        let Some(label_end) = content[label_start + 1..]
            .find(']')
            .map(|offset| label_start + 1 + offset)
        else {
            break;
        };
        if !content[label_end + 1..].starts_with('(') {
            idx = label_end + 1;
            continue;
        }
        let target_start = label_end + 2;
        let Some(target_end) = content[target_start..]
            .find(')')
            .map(|offset| target_start + offset)
        else {
            break;
        };
        let text = content[label_start + 1..label_end].trim();
        let target = content[target_start..target_end].trim();
        if is_local_markdown_target(target) {
            links.push(LinkCandidate {
                text: text.to_string(),
                target: target.to_string(),
                kind: GraphLinkKind::Markdown,
            });
        }
        idx = target_end + 1;
    }
}

fn is_local_markdown_target(target: &str) -> bool {
    let target = target.trim();
    !target.is_empty()
        && !target.starts_with('#')
        && !target.contains("://")
        && !target.starts_with("mailto:")
        && strip_anchor(target).ends_with(".md")
}

fn resolve_link_target(source_path: &str, candidate: &LinkCandidate) -> String {
    let target = strip_anchor(&candidate.target).trim();
    let target = match candidate.kind {
        GraphLinkKind::Wiki if !target.ends_with(".md") => format!("{target}.md"),
        _ => target.to_string(),
    };
    normalize_relative_target(source_path, &target)
}

fn strip_anchor(target: &str) -> &str {
    target.split('#').next().unwrap_or(target)
}

fn normalize_relative_target(source_path: &str, target: &str) -> String {
    let normalized = normalize_doc_path(target);
    if normalized.starts_with('/') {
        return normalized.trim_start_matches('/').to_string();
    }
    let base = source_path
        .rsplit_once('/')
        .map(|(dir, _)| dir)
        .unwrap_or_default();
    let joined = if base.is_empty() {
        normalized
    } else {
        format!("{base}/{normalized}")
    };
    collapse_path_segments(&joined)
}

fn collapse_path_segments(path: &str) -> String {
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_projection_resolves_wiki_and_markdown_links() {
        let a = DocId::from_u128(1);
        let b = DocId::from_u128(2);
        let docs = vec![
            GraphDocument {
                doc_id: a,
                path: "notes/a.md".to_string(),
                content: "[[b|B Note]] and [B](b.md) and [web](https://example.com)".to_string(),
            },
            GraphDocument {
                doc_id: b,
                path: "notes/b.md".to_string(),
                content: String::new(),
            },
        ];

        let graph = project_documents(&docs);

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 2);
        assert!(graph.edges.iter().all(|edge| edge.to_doc_id == b));
        assert!(
            graph
                .edges
                .iter()
                .all(|edge| edge.target_path == "notes/b.md")
        );
        assert!(graph.unresolved_links.is_empty());
    }

    #[test]
    fn graph_projection_reports_unresolved_without_creating_nodes() {
        let a = DocId::from_u128(1);
        let docs = vec![GraphDocument {
            doc_id: a,
            path: "notes/a.md".to_string(),
            content: "[[missing]]".to_string(),
        }];

        let graph = project_documents(&docs);

        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.edges.is_empty());
        assert_eq!(graph.unresolved_links.len(), 1);
        assert_eq!(graph.unresolved_links[0].target_path, "notes/missing.md");
    }

    #[test]
    fn graph_projection_normalizes_windows_and_parent_paths() {
        let a = DocId::from_u128(1);
        let b = DocId::from_u128(2);
        let docs = vec![
            GraphDocument {
                doc_id: a,
                path: "notes\\daily\\a.md".to_string(),
                content: "[B](../b.md#heading)".to_string(),
            },
            GraphDocument {
                doc_id: b,
                path: "notes/b.md".to_string(),
                content: String::new(),
            },
        ];

        let graph = project_documents(&docs);

        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].target_path, "notes/b.md");
        assert_eq!(graph.edges[0].to_doc_id, b);
    }
}
