// crates/core/src/tree/ops.rs
//! # 树操作辅助函数 (Tree Operations)
//!
//! 以 NodeId 为主键的迭代式树操作。

use super::node::FileNode;
use crate::models::{DocId, NodeId};
use std::collections::HashMap;

/// 节点信息 (内部使用)
#[derive(Clone, Debug)]
pub struct NodeInfo {
    pub node_id: NodeId,
    pub name: String,
    pub parent_id: Option<NodeId>,
    pub children_ids: Vec<NodeId>,
    pub path_cache: String,
    pub doc_id: Option<DocId>,
}

/// 迭代式删除节点及所有子节点
///
/// **不变量**: 删除后，node_id 及其所有子节点不存在于 nodes 中
pub fn remove_iterative(nodes: &mut HashMap<NodeId, NodeInfo>, node_id: NodeId) {
    let mut to_remove = Vec::new();
    let mut stack = vec![node_id];

    while let Some(current) = stack.pop() {
        to_remove.push(current);
        if let Some(info) = nodes.get(&current) {
            for child in &info.children_ids {
                stack.push(*child);
            }
        }
    }

    if let Some(info) = nodes.get(&node_id)
        && let Some(parent_id) = info.parent_id
        && let Some(parent) = nodes.get_mut(&parent_id)
    {
        parent.children_ids.retain(|c| *c != node_id);
    }

    for id in to_remove {
        nodes.remove(&id);
    }
}

/// 迭代式构建子树
///
/// **不变量**: 返回的 FileNode 树结构与 nodes 中的父子关系一致
pub fn build_subtree_iterative(
    nodes: &HashMap<NodeId, NodeInfo>,
    root_id: NodeId,
) -> Option<FileNode> {
    nodes.get(&root_id)?;

    let mut stack = vec![root_id];
    let mut visit_order = Vec::new();

    while let Some(id) = stack.pop() {
        visit_order.push(id);
        if let Some(info) = nodes.get(&id) {
            for child in info.children_ids.iter().rev() {
                stack.push(*child);
            }
        }
    }

    let mut built: HashMap<NodeId, FileNode> = HashMap::new();
    for id in visit_order.into_iter().rev() {
        let info = nodes.get(&id)?;
        let children: Vec<FileNode> = info
            .children_ids
            .iter()
            .filter_map(|c| built.remove(c))
            .collect();

        let mut node = FileNode {
            node_id: info.node_id,
            name: info.name.clone(),
            path: info.path_cache.clone(),
            doc_id: info.doc_id,
            children,
        };
        node.sort_children();
        built.insert(id, node);
    }

    built.remove(&root_id)
}
