// apps/web/src/hooks/use_core/apply.rs
//! # 树增量应用逻辑 (Tree Delta Application)
//!
//! 将 `TreeDelta` 应用到本地树结构。

use deve_core::models::NodeId;
use deve_core::tree::{FileNode, TreeDelta};

/// 将 TreeDelta 应用到现有树结构
pub fn apply_tree_delta(current: &mut Vec<FileNode>, delta: TreeDelta) {
    match delta {
        TreeDelta::Init { roots } => {
            *current = roots;
        }
        TreeDelta::Add {
            node_id,
            parent_id,
            name,
            path,
            doc_id,
        } => {
            let new_node = FileNode {
                node_id,
                name,
                path,
                doc_id,
                children: vec![],
            };
            insert_node(current, parent_id, new_node);
        }
        TreeDelta::Remove { node_id } => {
            remove_node(current, node_id);
        }
        TreeDelta::Update {
            node_id,
            parent_id,
            name,
            path,
        } => {
            if let Some(mut node) = remove_node_returning(current, node_id) {
                let old_path = node.path.clone();
                node.name = name;
                node.path = path.clone();
                update_children_paths(&mut node, &old_path, &path);
                insert_node(current, parent_id, node);
            }
        }
    }
}

/// 在指定父节点下插入新节点
fn insert_node(roots: &mut Vec<FileNode>, parent_id: Option<NodeId>, new_node: FileNode) {
    match parent_id {
        None => {
            roots.push(new_node);
            sort_nodes(roots);
        }
        Some(pid) => {
            if let Some(parent) = find_node_mut(roots, pid) {
                parent.children.push(new_node);
                sort_nodes(&mut parent.children);
            } else {
                roots.push(new_node);
                sort_nodes(roots);
            }
        }
    }
}

/// 移除并返回节点 (用于移动/重命名)
fn remove_node_returning(roots: &mut Vec<FileNode>, node_id: NodeId) -> Option<FileNode> {
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

/// 删除节点
fn remove_node(roots: &mut Vec<FileNode>, node_id: NodeId) {
    let _ = remove_node_returning(roots, node_id);
}

/// 递归更新子节点的路径前缀
fn update_children_paths(node: &mut FileNode, old_prefix: &str, new_prefix: &str) {
    let old_prefix = old_prefix.trim_end_matches('/');
    let old_prefix_slash = format!("{}/", old_prefix);
    for child in node.children.iter_mut() {
        if child.path.starts_with(&old_prefix_slash) {
            child.path = format!("{}{}", new_prefix, &child.path[old_prefix.len()..]);
        }
        update_children_paths(child, old_prefix, new_prefix);
    }
}

/// 排序节点 (文件夹优先，然后按字母顺序)
fn sort_nodes(nodes: &mut [FileNode]) {
    nodes.sort_by(|a, b| match (a.doc_id.is_none(), b.doc_id.is_none()) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
}

fn find_node_mut(roots: &mut [FileNode], node_id: NodeId) -> Option<&mut FileNode> {
    for node in roots.iter_mut() {
        if node.node_id == node_id {
            return Some(node);
        }
        if let Some(found) = find_node_mut(&mut node.children, node_id) {
            return Some(found);
        }
    }
    None
}
