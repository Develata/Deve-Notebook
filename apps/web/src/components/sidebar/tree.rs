// apps/web/src/components/sidebar/tree.rs
//! # 文件树组件逻辑 (File Tree Logic)
//!
//! 将扁平的文档列表转换为树结构（仅用于回退）。

#![allow(dead_code)]

use deve_core::models::{DocId, NodeId};
use deve_core::tree::FileNode;
use std::collections::BTreeMap;

// 构建树的内部辅助结构
struct TempNode {
    id: Option<DocId>,
    children: BTreeMap<String, TempNode>,
}

/// 构建文件树
///
/// **逻辑**:
/// 1. 遍历扁平的 (DocId, Path) 列表。
/// 2. 按 `/` 分割路径。
/// 3. 插入到中间的 `TempNode` 树中 (使用 BTreeMap 进行自动排序)。
/// 4. 递归地将 `TempNode` 转换为 `FileNode` 列表。
pub fn build_file_tree(docs: Vec<(DocId, String)>) -> Vec<FileNode> {
    let mut root = TempNode {
        id: None,
        children: BTreeMap::new(),
    };

    for (id, raw_path) in docs {
        let path = deve_core::utils::path::to_forward_slash(&raw_path);
        let parts: Vec<&str> = path.split('/').collect();
        let mut current = &mut root;

        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;

            current = current
                .children
                .entry(part.to_string())
                .or_insert_with(|| TempNode {
                    id: None,
                    children: BTreeMap::new(),
                });

            if is_last {
                current.id = Some(id);
            }
        }
    }

    fn convert(name: String, node: TempNode, path_prefix: String) -> FileNode {
        let full_path = if path_prefix.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", path_prefix, name)
        };
        let children: Vec<FileNode> = node
            .children
            .into_iter()
            .map(|(n, c)| convert(n, c, full_path.clone()))
            .collect();

        match node.id {
            Some(doc_id) => FileNode {
                node_id: NodeId::from_doc_id(doc_id),
                name,
                path: full_path,
                doc_id: Some(doc_id),
                children,
            },
            None => FileNode {
                node_id: NodeId::new(),
                name,
                path: full_path,
                doc_id: None,
                children,
            },
        }
    }

    root.children
        .into_iter()
        .map(|(n, c)| convert(n, c, "".to_string()))
        .collect()
}
