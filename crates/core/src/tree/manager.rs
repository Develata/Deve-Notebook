// crates/core/src/tree/manager.rs
//! # 树状态管理器 (Tree State Manager)
//!
//! 维护服务端的文件树状态，生成增量更新。
//! 仅在后端 (非 WASM) 环境中可用。

use super::delta::TreeDelta;
use super::node::FileNode;
use super::ops::{
    build_subtree_iterative, insert_path, remove_iterative, rename_children, split_path, NodeInfo,
};
use crate::models::DocId;
use crate::utils::path::to_forward_slash;
use std::collections::HashMap;

/// 树状态管理器
///
/// 维护内存中的文件树结构，提供高效的增量更新生成。
pub struct TreeManager {
    /// 路径 -> 节点信息 的映射 (O(1) 查找)
    nodes: HashMap<String, NodeInfo>,
}

impl TreeManager {
    /// 创建空的树管理器
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// 从文档列表初始化
    pub fn init_from_docs(&mut self, docs: Vec<(DocId, String)>) {
        self.nodes.clear();
        for (doc_id, path) in docs {
            insert_path(&mut self.nodes, &to_forward_slash(&path), Some(doc_id));
        }
    }

    /// 生成初始化 Delta
    pub fn build_init_delta(&self) -> TreeDelta {
        TreeDelta::init(self.build_tree_from_root())
    }

    // ========== 树操作 (生成 Delta) ==========

    /// 添加文件，返回 Delta
    pub fn add_file(&mut self, path: &str, doc_id: DocId) -> TreeDelta {
        let normalized = to_forward_slash(path);
        let (parent, name) = split_path(&normalized);
        let parent_str = parent.to_string();
        let name_str = name.to_string();
        insert_path(&mut self.nodes, &normalized, Some(doc_id));
        TreeDelta::add_file(normalized, parent_str, name_str, doc_id)
    }

    /// 添加文件夹，返回 Delta
    pub fn add_folder(&mut self, path: &str) -> TreeDelta {
        let normalized = to_forward_slash(path);
        let (parent, name) = split_path(&normalized);
        let parent_str = parent.to_string();
        let name_str = name.to_string();
        insert_path(&mut self.nodes, &normalized, None);
        TreeDelta::add_folder(normalized, parent_str, name_str)
    }

    /// 删除节点，返回 Delta
    pub fn remove(&mut self, path: &str) -> TreeDelta {
        let normalized = to_forward_slash(path);
        remove_iterative(&mut self.nodes, &normalized);
        TreeDelta::remove(normalized)
    }

    /// 重命名/移动节点，返回 Delta
    pub fn rename(&mut self, old_path: &str, new_path: &str) -> TreeDelta {
        let old = to_forward_slash(old_path);
        let new = to_forward_slash(new_path);

        if let Some(mut info) = self.nodes.remove(&old) {
            let (new_parent, new_name) = split_path(&new);
            info.name = new_name.to_string();
            info.parent_path = new_parent.to_string();
            rename_children(&mut self.nodes, &old, &new);
            self.nodes.insert(new.clone(), info);
        }
        TreeDelta::rename(old, new)
    }

    /// 从根节点构建完整树 (迭代实现)
    fn build_tree_from_root(&self) -> Vec<FileNode> {
        let mut roots: Vec<FileNode> = self
            .nodes
            .iter()
            .filter(|(_, info)| info.parent_path.is_empty())
            .filter_map(|(path, _)| build_subtree_iterative(&self.nodes, path))
            .collect();

        roots.sort_by(
            |a: &FileNode, b: &FileNode| match (a.is_folder(), b.is_folder()) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            },
        );

        roots
    }
}

impl Default for TreeManager {
    fn default() -> Self {
        Self::new()
    }
}
