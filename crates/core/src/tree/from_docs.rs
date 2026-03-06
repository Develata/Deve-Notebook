//! # 文档列表树构建 (Doc-list Tree Builder)
//! 将 (DocId, path) 列表转换为内存树结构。仅用于只读/临时场景。

use super::ops::NodeInfo;
use crate::models::{DocId, NodeId};
use crate::utils::path::to_forward_slash;
use std::collections::HashMap;

pub fn build_tree_from_docs(nodes: &mut HashMap<NodeId, NodeInfo>, docs: Vec<(DocId, String)>) {
    nodes.clear();
    let mut dir_ids: HashMap<String, NodeId> = HashMap::new();

    for (doc_id, raw_path) in docs {
        let path = to_forward_slash(&raw_path);
        let (parent_path, name) = split_path(&path);
        let parent_id = ensure_dir_nodes(nodes, &mut dir_ids, parent_path);

        let node_id = NodeId::from_doc_id(doc_id);
        if nodes.contains_key(&node_id) {
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
        insert_node(nodes, info);
    }
}

fn insert_node(nodes: &mut HashMap<NodeId, NodeInfo>, info: NodeInfo) {
    let node_id = info.node_id;
    let parent_id = info.parent_id;
    nodes.insert(node_id, info);

    if let Some(pid) = parent_id
        && let Some(parent) = nodes.get_mut(&pid)
        && !parent.children_ids.contains(&node_id)
    {
        parent.children_ids.push(node_id);
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

    let mut current = String::new();
    let mut parent_id: Option<NodeId> = None;
    for part in path.split('/') {
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
