//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!
use deve_core::models::NodeId;
use deve_core::tree::FileNode;
use std::collections::HashSet;

use super::tree_lookup::{
    find_node_mut, find_node_mut_by_path, normalize_path, parent_path_of, sort_nodes,
};

pub(super) fn insert_node(
    roots: &mut Vec<FileNode>,
    parent_id: Option<NodeId>,
    new_node: FileNode,
) {
    match parent_id {
        None => {
            roots.push(new_node);
            sort_nodes(roots);
        }
        Some(pid) => {
            if let Some(parent) = find_node_mut(roots, pid) {
                parent.children.push(new_node);
                sort_nodes(&mut parent.children);
            } else if let Some(parent_path) = parent_path_of(&new_node.path) {
                if let Some(parent) = find_node_mut_by_path(roots, parent_path) {
                    parent.children.push(new_node);
                    sort_nodes(&mut parent.children);
                }
            } else {
                roots.push(new_node);
                sort_nodes(roots);
            }
        }
    }
}

pub(super) fn remove_node_returning(
    roots: &mut Vec<FileNode>,
    node_id: NodeId,
) -> Option<FileNode> {
    if let Some(idx) = roots.iter().position(|n| n.node_id == node_id) {
        return Some(roots.remove(idx));
    }
    for node in roots.iter_mut() {
        if let Some(found) = remove_node_returning(&mut node.children, node_id) {
            return Some(found);
        }
    }
    None
}

pub(super) fn remove_node(roots: &mut Vec<FileNode>, node_id: NodeId) {
    let _ = remove_node_returning(roots, node_id);
}

pub(super) fn remove_node_by_path(roots: &mut Vec<FileNode>, path: &str) -> Option<FileNode> {
    if let Some(idx) = roots.iter().position(|n| n.path == path) {
        return Some(roots.remove(idx));
    }
    for node in roots.iter_mut() {
        if let Some(found) = remove_node_by_path(&mut node.children, path) {
            return Some(found);
        }
    }
    None
}

pub(super) fn update_children_paths(node: &mut FileNode, old_prefix: &str, new_prefix: &str) {
    let old_prefix = old_prefix.trim_end_matches('/');
    let old_prefix_slash = format!("{}/", old_prefix);

    for child in node.children.iter_mut() {
        if child.path.starts_with(&old_prefix_slash) {
            child.path = format!("{}{}", new_prefix, &child.path[old_prefix.len()..]);
        }
        update_children_paths(child, old_prefix, new_prefix);
    }
}

pub(super) fn dedupe_tree(
    nodes: &mut Vec<FileNode>,
    seen_ids: &mut HashSet<NodeId>,
    seen_paths: &mut HashSet<String>,
) {
    nodes.retain(|node| {
        let path_key = normalize_path(&node.path);
        seen_ids.insert(node.node_id) && seen_paths.insert(path_key)
    });

    for node in nodes.iter_mut() {
        dedupe_tree(&mut node.children, seen_ids, seen_paths);
    }
    sort_nodes(nodes);
}
