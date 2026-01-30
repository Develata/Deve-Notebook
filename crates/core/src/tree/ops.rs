// crates/core/src/tree/ops.rs
//! # 树操作辅助函数 (Tree Operations)
//!
//! 提供文件树的迭代式操作实现。
//! **关键**: 所有遍历操作使用迭代而非递归，避免深层文件树导致栈溢出。

use super::node::FileNode;
use crate::models::DocId;
use std::collections::HashMap;

/// 节点信息 (内部使用)
#[derive(Clone, Debug)]
pub struct NodeInfo {
    pub name: String,
    pub doc_id: Option<DocId>,
    pub parent_path: String,
    pub children_paths: Vec<String>,
}

/// 迭代式删除节点及所有子节点
///
/// **不变量**: 删除后，path 及其所有子路径都不存在于 nodes 中
///
/// **实现**: 使用显式栈代替递归，O(n) 时间，O(depth) 堆内存
pub fn remove_iterative(nodes: &mut HashMap<String, NodeInfo>, path: &str) {
    // 收集所有需要删除的路径 (BFS)
    let mut to_remove = Vec::new();
    let mut stack = vec![path.to_string()];

    while let Some(current) = stack.pop() {
        to_remove.push(current.clone());
        if let Some(info) = nodes.get(&current) {
            for child in &info.children_paths {
                stack.push(child.clone());
            }
        }
    }

    // 从父节点的 children 中移除
    if let Some(info) = nodes.get(path) {
        let parent = info.parent_path.clone();
        if let Some(p) = nodes.get_mut(&parent) {
            p.children_paths.retain(|c| c != path);
        }
    }

    // 批量删除
    for p in to_remove {
        nodes.remove(&p);
    }
}

/// 迭代式构建子树
///
/// **不变量**: 返回的 FileNode 树结构与 nodes 中的父子关系一致
///
/// **实现**: 后序遍历 (先构建叶子，再组装父节点)
pub fn build_subtree_iterative(
    nodes: &HashMap<String, NodeInfo>,
    root_path: &str,
) -> Option<FileNode> {
    nodes.get(root_path)?;

    // 收集所有需要构建的路径 (DFS 顺序)
    let mut stack = vec![root_path.to_string()];
    let mut visit_order = Vec::new();

    while let Some(path) = stack.pop() {
        visit_order.push(path.clone());
        if let Some(info) = nodes.get(&path) {
            for child in info.children_paths.iter().rev() {
                stack.push(child.clone());
            }
        }
    }

    // 后序构建: 从叶子到根
    let mut built: HashMap<String, FileNode> = HashMap::new();

    for path in visit_order.into_iter().rev() {
        let info = nodes.get(&path)?;
        let children: Vec<FileNode> = info
            .children_paths
            .iter()
            .filter_map(|c| built.remove(c))
            .collect();

        let mut node = FileNode {
            name: info.name.clone(),
            path: path.clone(),
            doc_id: info.doc_id,
            children,
        };
        node.sort_children();
        built.insert(path, node);
    }

    built.remove(root_path)
}

/// 分割路径为 (父路径, 名称)
pub fn split_path(path: &str) -> (&str, &str) {
    path.rfind('/')
        .map_or(("", path), |pos| (&path[..pos], &path[pos + 1..]))
}

/// 插入路径，自动创建中间文件夹
///
/// **不变量**: 路径上的所有中间目录都会被创建
pub fn insert_path(nodes: &mut HashMap<String, NodeInfo>, path: &str, doc_id: Option<DocId>) {
    let parts: Vec<&str> = path.split('/').collect();
    let mut current = String::new();
    let mut parent = String::new();

    for (i, part) in parts.iter().enumerate() {
        if !current.is_empty() {
            current.push('/');
        }
        current.push_str(part);

        let is_last = i == parts.len() - 1;
        let id = if is_last { doc_id } else { None };

        if !nodes.contains_key(&current) {
            nodes.insert(
                current.clone(),
                NodeInfo {
                    name: part.to_string(),
                    doc_id: id,
                    parent_path: parent.clone(),
                    children_paths: Vec::new(),
                },
            );
            if !parent.is_empty()
                && let Some(p) = nodes.get_mut(&parent)
                    && !p.children_paths.contains(&current) {
                        p.children_paths.push(current.clone());
                    }
        } else if is_last
            && let Some(node) = nodes.get_mut(&current) {
                node.doc_id = id;
            }
        parent = current.clone();
    }
}

/// 迭代式更新子节点路径 (重命名/移动)
pub fn rename_children(nodes: &mut HashMap<String, NodeInfo>, old_prefix: &str, new_prefix: &str) {
    let prefix_pattern = format!("{}/", old_prefix);
    let affected: Vec<(String, String)> = nodes
        .keys()
        .filter(|p| p.starts_with(&prefix_pattern))
        .map(|old_key| {
            let new_key = format!("{}{}", new_prefix, &old_key[old_prefix.len()..]);
            (old_key.clone(), new_key)
        })
        .collect();

    for (old_key, new_key) in affected {
        if let Some(mut info) = nodes.remove(&old_key) {
            let (new_parent, _) = split_path(&new_key);
            info.parent_path = new_parent.to_string();
            nodes.insert(new_key, info);
        }
    }
}
