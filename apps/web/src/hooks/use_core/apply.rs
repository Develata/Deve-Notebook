// apps/web/src/hooks/use_core/apply.rs
//! # 树增量应用逻辑 (Tree Delta Application)
//!
//! 将 `TreeDelta` 应用到本地树结构。

use deve_core::models::DocId;
use deve_core::tree::{FileNode, TreeDelta};

/// 将 TreeDelta 应用到现有树结构
pub fn apply_tree_delta(current: &mut Vec<FileNode>, delta: TreeDelta) {
    match delta {
        TreeDelta::Init { roots } => {
            // 完全替换
            *current = roots;
        }
        TreeDelta::Add {
            path,
            parent_path,
            name,
            doc_id,
        } => {
            let new_node = FileNode {
                name,
                path,
                doc_id,
                children: vec![],
            };
            insert_node(current, &parent_path, new_node);
        }
        TreeDelta::Remove { path } => {
            remove_node(current, &path);
        }
        TreeDelta::Rename { old_path, new_path } => {
            let (old_parent, _) = split_path(&old_path);
            let (new_parent, _) = split_path(&new_path);

            if old_parent == new_parent {
                // Same directory: simply rename in place
                rename_node_in_place(current, &old_path, &new_path);
            } else {
                // Move operation: Remove from old location, insert to new
                // We need to extract the node to preserve doc_id and children
                if let Some(mut node) = remove_node_returning(current, &old_path) {
                    // Update node properties for new location
                    node.path = new_path.clone();
                    // Extract new name
                    if let Some(pos) = new_path.rfind('/') {
                        node.name = new_path[pos + 1..].to_string();
                    } else {
                        node.name = new_path.clone();
                    }

                    // Recursively update children paths
                    update_children_paths(&mut node, &old_path, &new_path);

                    // Insert into new parent
                    insert_node(current, new_parent, node);
                }
            }
        }
    }
}

/// 在指定父路径下插入新节点 (Iterative approach)
fn insert_node(roots: &mut Vec<FileNode>, parent_path: &str, new_node: FileNode) {
    if parent_path.is_empty() {
        roots.push(new_node);
        sort_nodes(roots);
        return;
    }

    let parts: Vec<&str> = parent_path.split('/').collect();
    let mut current_level = roots;
    let mut current_path = String::new();

    for part in parts {
        if !current_path.is_empty() {
            current_path.push('/');
        }
        current_path.push_str(part);

        // 查找当前层级是否包含该部分
        let idx = current_level.iter().position(|n| n.name == part);

        match idx {
            Some(i) => {
                // 进入子目录
                current_level = &mut current_level[i].children;
            }
            None => {
                // 创建新文件夹 (Implicit folder creation)
                let folder = FileNode::folder(part.to_string(), current_path.clone());
                current_level.push(folder);
                let last_idx = current_level.len() - 1;
                current_level = &mut current_level[last_idx].children;
            }
        }
    }

    // 此时 current_level 即为目标父节点的 children 列表
    current_level.push(new_node);
    sort_nodes(current_level);
}

/// 递归删除指定路径的节点 (Standard remove)
fn remove_node(roots: &mut Vec<FileNode>, path: &str) {
    remove_node_returning(roots, path);
}

/// 移除并返回节点 (用于移动操作)
fn remove_node_returning(roots: &mut Vec<FileNode>, path: &str) -> Option<FileNode> {
    // Check current level
    if let Some(idx) = roots.iter().position(|n| n.path == path) {
        return Some(roots.remove(idx));
    }

    // Recurse
    for node in roots.iter_mut() {
        if path.starts_with(&format!("{}/", node.path)) {
            return remove_node_returning(&mut node.children, path);
        }
    }
    None
}

/// 原地重命名 (同级目录)
fn rename_node_in_place(roots: &mut Vec<FileNode>, old_path: &str, new_path: &str) {
    for node in roots.iter_mut() {
        if node.path == old_path {
            node.path = new_path.to_string();
            if let Some(pos) = new_path.rfind('/') {
                node.name = new_path[pos + 1..].to_string();
            } else {
                node.name = new_path.to_string();
            }
            update_children_paths(node, old_path, new_path);
            return;
        }
        if old_path.starts_with(&format!("{}/", node.path)) {
            rename_node_in_place(&mut node.children, old_path, new_path);
            return;
        }
    }
}

/// 递归更新子节点的路径前缀
fn update_children_paths(node: &mut FileNode, old_prefix: &str, new_prefix: &str) {
    for child in node.children.iter_mut() {
        if child.path.starts_with(old_prefix) {
            child.path = format!("{}{}", new_prefix, &child.path[old_prefix.len()..]);
        }
        update_children_paths(child, old_prefix, new_prefix);
    }
}

/// 排序节点 (文件夹优先，然后按字母顺序)
fn sort_nodes(nodes: &mut Vec<FileNode>) {
    nodes.sort_by(|a, b| {
        match (a.doc_id.is_none(), b.doc_id.is_none()) {
            (true, false) => std::cmp::Ordering::Less, // 文件夹优先
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });
}

/// 辅助函数: 分割路径
fn split_path(path: &str) -> (&str, &str) {
    match path.rfind('/') {
        Some(pos) => (&path[..pos], &path[pos + 1..]),
        None => ("", path),
    }
}
