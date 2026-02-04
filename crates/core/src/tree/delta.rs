// crates/core/src/tree/delta.rs
//! # 树增量更新类型 (Tree Delta Types)
//!
//! 定义树的增量更新消息类型，用于 WebSocket 传输。

use super::node::FileNode;
use crate::models::{DocId, NodeId};
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
        /// 新节点 ID
        node_id: NodeId,
        /// 父节点 ID (None 表示根目录)
        parent_id: Option<NodeId>,
        /// 节点名称
        name: String,
        /// 完整路径 (缓存)
        path: String,
        /// 文档 ID (文件夹为 None)
        doc_id: Option<DocId>,
    },

    /// 删除节点
    ///
    /// 删除文件或文件夹时发送。
    Remove {
        /// 被删除节点 ID
        node_id: NodeId,
    },

    /// 重命名/移动节点
    ///
    /// 重命名或移动文件/文件夹时发送。
    Update {
        /// 目标节点 ID
        node_id: NodeId,
        /// 新父节点 ID (None 表示根目录)
        parent_id: Option<NodeId>,
        /// 新名称
        name: String,
        /// 新路径 (缓存)
        path: String,
    },
}

impl TreeDelta {
    /// 创建初始化 Delta
    pub fn init(roots: Vec<FileNode>) -> Self {
        Self::Init { roots }
    }

    /// 创建添加文件 Delta
    pub fn add_file(
        node_id: NodeId,
        parent_id: Option<NodeId>,
        name: String,
        path: String,
        doc_id: DocId,
    ) -> Self {
        Self::Add {
            node_id,
            parent_id,
            name,
            path,
            doc_id: Some(doc_id),
        }
    }

    /// 创建添加文件夹 Delta
    pub fn add_folder(
        node_id: NodeId,
        parent_id: Option<NodeId>,
        name: String,
        path: String,
    ) -> Self {
        Self::Add {
            node_id,
            parent_id,
            name,
            path,
            doc_id: None,
        }
    }

    /// 创建删除 Delta
    pub fn remove(node_id: NodeId) -> Self {
        Self::Remove { node_id }
    }

    /// 创建重命名 Delta
    pub fn update(node_id: NodeId, parent_id: Option<NodeId>, name: String, path: String) -> Self {
        Self::Update {
            node_id,
            parent_id,
            name,
            path,
        }
    }
}
