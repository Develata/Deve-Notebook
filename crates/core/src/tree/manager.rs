// crates/core/src/tree/manager.rs
//! # 树状态管理器 (Tree State Manager)
//!
//! 维护服务端的文件树状态，生成增量更新。
//! 仅在后端 (非 WASM) 环境中可用。

use super::delta::TreeDelta;
use super::node::FileNode;
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

/// 节点信息 (内部使用)
#[derive(Clone, Debug)]
struct NodeInfo {
    name: String,
    doc_id: Option<DocId>,
    parent_path: String,
    children_paths: Vec<String>,
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
            self.insert_path(&to_forward_slash(&path), Some(doc_id));
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
        self.insert_path(&normalized, Some(doc_id));
        TreeDelta::add_file(normalized, parent_str, name_str, doc_id)
    }

    /// 添加文件夹，返回 Delta
    pub fn add_folder(&mut self, path: &str) -> TreeDelta {
        let normalized = to_forward_slash(path);
        let (parent, name) = split_path(&normalized);
        let parent_str = parent.to_string();
        let name_str = name.to_string();
        self.insert_path(&normalized, None);
        TreeDelta::add_folder(normalized, parent_str, name_str)
    }

    /// 删除节点，返回 Delta
    pub fn remove(&mut self, path: &str) -> TreeDelta {
        let normalized = to_forward_slash(path);
        self.remove_recursive(&normalized);
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
            self.rename_children(&old, &new);
            self.nodes.insert(new.clone(), info);
        }
        TreeDelta::rename(old, new)
    }

    // ========== 内部方法 ==========

    /// 插入路径，自动创建中间文件夹
    fn insert_path(&mut self, path: &str, doc_id: Option<DocId>) {
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

            if !self.nodes.contains_key(&current) {
                self.nodes.insert(
                    current.clone(),
                    NodeInfo {
                        name: part.to_string(),
                        doc_id: id,
                        parent_path: parent.clone(),
                        children_paths: Vec::new(),
                    },
                );
                if !parent.is_empty() {
                    if let Some(p) = self.nodes.get_mut(&parent) {
                        if !p.children_paths.contains(&current) {
                            p.children_paths.push(current.clone());
                        }
                    }
                }
            } else if is_last {
                if let Some(node) = self.nodes.get_mut(&current) {
                    node.doc_id = id;
                }
            }
            parent = current.clone();
        }
    }

    /// 递归删除节点及子节点
    fn remove_recursive(&mut self, path: &str) {
        let children: Vec<String> = self
            .nodes
            .get(path)
            .map(|i| i.children_paths.clone())
            .unwrap_or_default();

        for child in children {
            self.remove_recursive(&child);
        }

        if let Some(info) = self.nodes.get(path) {
            let parent = info.parent_path.clone();
            if let Some(p) = self.nodes.get_mut(&parent) {
                p.children_paths.retain(|c| c != path);
            }
        }
        self.nodes.remove(path);
    }

    /// 递归更新子节点路径
    fn rename_children(&mut self, old_prefix: &str, new_prefix: &str) {
        let affected: Vec<(String, String)> = self
            .nodes
            .keys()
            .filter(|p| p.starts_with(&format!("{}/", old_prefix)))
            .map(|old| {
                (
                    old.clone(),
                    format!("{}{}", new_prefix, &old[old_prefix.len()..]),
                )
            })
            .collect();

        for (old, new) in affected {
            if let Some(mut info) = self.nodes.remove(&old) {
                let (new_parent, _) = split_path(&new);
                info.parent_path = new_parent.to_string();
                self.nodes.insert(new, info);
            }
        }
    }

    /// 从根节点构建完整树
    fn build_tree_from_root(&self) -> Vec<FileNode> {
        self.nodes
            .iter()
            .filter(|(_, info)| info.parent_path.is_empty())
            .filter_map(|(path, _)| self.build_subtree(path))
            .collect()
    }

    /// 递归构建子树
    fn build_subtree(&self, path: &str) -> Option<FileNode> {
        let info = self.nodes.get(path)?;
        let mut node = FileNode {
            name: info.name.clone(),
            path: path.to_string(),
            doc_id: info.doc_id,
            children: info
                .children_paths
                .iter()
                .filter_map(|c| self.build_subtree(c))
                .collect(),
        };
        node.sort_children();
        Some(node)
    }
}

impl Default for TreeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 分割路径为 (父路径, 名称)
fn split_path(path: &str) -> (&str, &str) {
    path.rfind('/')
        .map_or(("", path), |pos| (&path[..pos], &path[pos + 1..]))
}
