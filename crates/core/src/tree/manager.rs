// crates/core/src/tree/manager.rs
//! # 树状态管理器 (Tree State Manager)
//!
//! 维护服务端的文件树状态，生成增量更新。
//! 仅在后端 (非 WASM) 环境中可用。

use super::delta::TreeDelta;
use super::node::FileNode;
use super::ops::{NodeInfo, build_subtree_iterative, remove_iterative};
use crate::models::{DocId, NodeId, NodeMeta};
use crate::utils::path::to_forward_slash;
use std::collections::HashMap;

/// 树状态管理器
///
/// 维护内存中的文件树结构，提供高效的增量更新生成。
pub struct TreeManager {
    /// NodeId -> 节点信息 的映射 (O(1) 查找)
    nodes: HashMap<NodeId, NodeInfo>,
}

impl TreeManager {
    /// 创建空的树管理器
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn has_node(&self, node_id: NodeId) -> bool {
        self.nodes.contains_key(&node_id)
    }

    /// 从 Node 列表初始化
    ///
    /// # Invariants
    /// - NodeId 全局唯一
    /// - parent_id 构成树形结构 (无环)
    pub fn init_from_nodes(&mut self, nodes: Vec<(NodeId, NodeMeta)>) {
        self.nodes.clear();

        for (node_id, meta) in nodes.iter() {
            self.nodes.insert(
                *node_id,
                NodeInfo {
                    node_id: *node_id,
                    name: meta.name.clone(),
                    parent_id: meta.parent_id,
                    children_ids: Vec::new(),
                    path_cache: meta.path.clone(),
                    doc_id: meta.doc_id,
                },
            );
        }

        for (node_id, meta) in nodes {
            if let Some(parent_id) = meta.parent_id
                && let Some(parent) = self.nodes.get_mut(&parent_id)
                && !parent.children_ids.contains(&node_id)
            {
                parent.children_ids.push(node_id);
            }
        }
    }

    /// 从文档列表初始化 (只用于只读/临时场景)
    pub fn init_from_docs(&mut self, docs: Vec<(DocId, String)>) {
        self.nodes.clear();
        let mut dir_ids: HashMap<String, NodeId> = HashMap::new();

        for (doc_id, raw_path) in docs {
            let path = to_forward_slash(&raw_path);
            let (parent_path, name) = split_path(&path);
            let parent_id = ensure_dir_nodes(&mut self.nodes, &mut dir_ids, parent_path);

            let node_id = NodeId::from_doc_id(doc_id);
            if self.nodes.contains_key(&node_id) {
                continue;
            }
            let info = NodeInfo {
                node_id,
                name: name.to_string(),
                parent_id,
                children_ids: Vec::new(),
                path_cache: path.clone(),
                doc_id: Some(doc_id),
            };
            self.insert_node(info);
        }
    }

    /// 生成初始化 Delta
    pub fn build_init_delta(&self) -> TreeDelta {
        TreeDelta::init(self.build_tree_from_root())
    }

    // ========== 树操作 (生成 Delta) ==========

    /// 添加文件节点，返回 Delta
    pub fn add_file(
        &mut self,
        node_id: NodeId,
        path: String,
        parent_id: Option<NodeId>,
        name: String,
        doc_id: DocId,
    ) -> TreeDelta {
        self.insert_node(NodeInfo {
            node_id,
            name: name.clone(),
            parent_id,
            children_ids: Vec::new(),
            path_cache: path.clone(),
            doc_id: Some(doc_id),
        });
        TreeDelta::add_file(node_id, parent_id, name, path, doc_id)
    }

    /// 添加文件夹节点，返回 Delta
    pub fn add_folder(
        &mut self,
        node_id: NodeId,
        path: String,
        parent_id: Option<NodeId>,
        name: String,
    ) -> TreeDelta {
        self.insert_node(NodeInfo {
            node_id,
            name: name.clone(),
            parent_id,
            children_ids: Vec::new(),
            path_cache: path.clone(),
            doc_id: None,
        });
        TreeDelta::add_folder(node_id, parent_id, name, path)
    }

    /// 删除节点，返回 Delta
    pub fn remove(&mut self, node_id: NodeId) -> TreeDelta {
        remove_iterative(&mut self.nodes, node_id);
        TreeDelta::remove(node_id)
    }

    /// 更新节点 (重命名/移动)
    pub fn update_node(
        &mut self,
        node_id: NodeId,
        new_parent_id: Option<NodeId>,
        new_name: String,
        new_path: String,
    ) -> TreeDelta {
        let (old_parent, old_path) = match self.nodes.get(&node_id) {
            Some(info) => (info.parent_id, info.path_cache.clone()),
            None => return TreeDelta::update(node_id, new_parent_id, new_name, new_path),
        };

        if old_parent != new_parent_id {
            if let Some(parent_id) = old_parent
                && let Some(parent) = self.nodes.get_mut(&parent_id)
            {
                parent.children_ids.retain(|c| *c != node_id);
            }
            if let Some(parent_id) = new_parent_id
                && let Some(parent) = self.nodes.get_mut(&parent_id)
                && !parent.children_ids.contains(&node_id)
            {
                parent.children_ids.push(node_id);
            }
        }

        if let Some(info) = self.nodes.get_mut(&node_id) {
            info.parent_id = new_parent_id;
            info.name = new_name.clone();
            info.path_cache = new_path.clone();
        }
        self.update_subtree_paths(&old_path, &new_path);

        TreeDelta::update(node_id, new_parent_id, new_name, new_path)
    }

    /// 从根节点构建完整树 (迭代实现)
    fn build_tree_from_root(&self) -> Vec<FileNode> {
        let mut roots: Vec<FileNode> = self
            .nodes
            .iter()
            .filter(|(_, info)| info.parent_id.is_none())
            .filter_map(|(id, _)| build_subtree_iterative(&self.nodes, *id))
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

    fn insert_node(&mut self, info: NodeInfo) {
        let node_id = info.node_id;
        let parent_id = info.parent_id;
        self.nodes.insert(node_id, info);

        if let Some(pid) = parent_id
            && let Some(parent) = self.nodes.get_mut(&pid)
            && !parent.children_ids.contains(&node_id)
        {
            parent.children_ids.push(node_id);
        }
    }

    fn update_subtree_paths(&mut self, old_prefix: &str, new_prefix: &str) {
        let old_prefix = old_prefix.trim_end_matches('/');
        if old_prefix.is_empty() {
            return;
        }
        let old_prefix_slash = format!("{}/", old_prefix);
        for info in self.nodes.values_mut() {
            if info.path_cache.starts_with(&old_prefix_slash) {
                info.path_cache = format!("{}{}", new_prefix, &info.path_cache[old_prefix.len()..]);
            }
        }
    }
}

fn ensure_dir_nodes(
    nodes: &mut HashMap<NodeId, NodeInfo>,
    dir_ids: &mut HashMap<String, NodeId>,
    path: &str,
) -> Option<NodeId> {
    if path.is_empty() {
        return None;
    }

    let parts: Vec<&str> = path.split('/').collect();
    let mut current = String::new();
    let mut parent_id: Option<NodeId> = None;

    for part in parts {
        if part.is_empty() {
            continue;
        }
        if !current.is_empty() {
            current.push('/');
        }
        current.push_str(part);

        let node_id = if let Some(existing) = dir_ids.get(&current) {
            *existing
        } else {
            let node_id = NodeId::new();
            let info = NodeInfo {
                node_id,
                name: part.to_string(),
                parent_id,
                children_ids: Vec::new(),
                path_cache: current.clone(),
                doc_id: None,
            };
            nodes.insert(node_id, info);
            dir_ids.insert(current.clone(), node_id);
            node_id
        };

        if let Some(pid) = parent_id
            && let Some(parent) = nodes.get_mut(&pid)
            && !parent.children_ids.contains(&node_id)
        {
            parent.children_ids.push(node_id);
        }

        parent_id = Some(node_id);
    }

    parent_id
}

fn split_path(path: &str) -> (&str, &str) {
    path.rfind('/')
        .map_or(("", path), |pos| (&path[..pos], &path[pos + 1..]))
}

impl Default for TreeManager {
    fn default() -> Self {
        Self::new()
    }
}
