// crates/core/src/tree/node.rs
//! # 文件节点定义 (File Node Definition)
//!
//! 定义文件树中的节点结构，用于表示文件和文件夹。

use crate::models::DocId;
use serde::{Deserialize, Serialize};

/// 文件树节点
///
/// 代表侧边栏树中的一个文件或文件夹。
/// 该结构可序列化，用于 WebSocket 传输。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileNode {
    /// 节点名称 (文件名或文件夹名)
    pub name: String,

    /// 完整路径 (使用正斜杠格式)
    pub path: String,

    /// 文档 ID
    /// - `Some(id)`: 文件节点
    /// - `None`: 文件夹节点
    pub doc_id: Option<DocId>,

    /// 子节点列表
    /// 对于文件节点，此列表为空
    pub children: Vec<FileNode>,
}

impl FileNode {
    /// 创建文件节点
    pub fn file(name: String, path: String, doc_id: DocId) -> Self {
        Self {
            name,
            path,
            doc_id: Some(doc_id),
            children: Vec::new(),
        }
    }

    /// 创建文件夹节点
    pub fn folder(name: String, path: String) -> Self {
        Self {
            name,
            path,
            doc_id: None,
            children: Vec::new(),
        }
    }

    /// 判断是否为文件夹
    pub fn is_folder(&self) -> bool {
        self.doc_id.is_none()
    }

    /// 判断是否为文件
    pub fn is_file(&self) -> bool {
        self.doc_id.is_some()
    }

    /// 添加子节点
    pub fn add_child(&mut self, child: FileNode) {
        self.children.push(child);
    }

    /// 按名称排序子节点 (文件夹优先，然后按字母顺序)
    pub fn sort_children(&mut self) {
        self.children
            .sort_by(|a, b| match (a.is_folder(), b.is_folder()) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            });
        // 递归排序
        for child in &mut self.children {
            child.sort_children();
        }
    }
}
