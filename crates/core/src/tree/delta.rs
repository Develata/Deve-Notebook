// crates/core/src/tree/delta.rs
//! # 树增量更新类型 (Tree Delta Types)
//!
//! 定义树的增量更新消息类型，用于 WebSocket 传输。

use super::node::FileNode;
use crate::models::DocId;
use serde::{Deserialize, Serialize};

/// 树增量更新
///
/// 表示文件树的一次变更操作。
/// 前端收到后只需 O(1) 应用，无需重建整棵树。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TreeDelta {
    /// 初始化完整树
    ///
    /// 首次连接或重新同步时发送完整树结构。
    Init {
        /// 根节点的子节点列表
        roots: Vec<FileNode>,
    },

    /// 添加节点
    ///
    /// 新建文件或文件夹时发送。
    Add {
        /// 新节点的完整路径
        path: String,
        /// 父节点路径 (空字符串表示根目录)
        parent_path: String,
        /// 节点名称
        name: String,
        /// 文档 ID (文件夹为 None)
        doc_id: Option<DocId>,
    },

    /// 删除节点
    ///
    /// 删除文件或文件夹时发送。
    Remove {
        /// 被删除节点的路径
        path: String,
    },

    /// 重命名/移动节点
    ///
    /// 重命名或移动文件/文件夹时发送。
    Rename {
        /// 原路径
        old_path: String,
        /// 新路径
        new_path: String,
    },
}

impl TreeDelta {
    /// 创建初始化 Delta
    pub fn init(roots: Vec<FileNode>) -> Self {
        Self::Init { roots }
    }

    /// 创建添加文件 Delta
    pub fn add_file(path: String, parent_path: String, name: String, doc_id: DocId) -> Self {
        Self::Add {
            path,
            parent_path,
            name,
            doc_id: Some(doc_id),
        }
    }

    /// 创建添加文件夹 Delta
    pub fn add_folder(path: String, parent_path: String, name: String) -> Self {
        Self::Add {
            path,
            parent_path,
            name,
            doc_id: None,
        }
    }

    /// 创建删除 Delta
    pub fn remove(path: String) -> Self {
        Self::Remove { path }
    }

    /// 创建重命名 Delta
    pub fn rename(old_path: String, new_path: String) -> Self {
        Self::Rename { old_path, new_path }
    }
}
