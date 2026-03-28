// apps/web/src/hooks/use_core/apply.rs
//! # 树增量应用逻辑 (Tree Delta Application)
//!
//! 将 `TreeDelta` 应用到本地树结构。

#[path = "apply_tree_nodes.rs"]
mod tree_nodes;

use deve_core::tree::{FileNode, TreeDelta};
use std::collections::HashSet;

use self::tree_nodes::{
    dedupe_tree, insert_node, remove_node, remove_node_by_path, remove_node_returning,
    update_children_paths,
};

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
            remove_node(current, node_id);
            remove_node_by_path(current, &path);
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
                remove_node_by_path(current, &path);
                insert_node(current, parent_id, node);
            }
        }
    }

    dedupe_tree(current, &mut HashSet::new(), &mut HashSet::new());
}
