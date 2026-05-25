//! # 账本事件模型
//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!
//! Invariants:
//! - `ContentOp` 只描述文本内容变化。
//! - `StructureOp` 只描述节点/路径结构变化。
//! - `LedgerEntry` 的权威载荷永远是 `LedgerEvent`，而不是分散副作用。

use super::{DocId, NodeId, PeerId};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentOp {
    Insert { pos: u32, content: SmolStr },
    Delete { pos: u32, len: u32 },
}

pub type Op = ContentOp;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructureOp {
    CreateFile {
        node_id: NodeId,
        doc_id: DocId,
        parent_id: Option<NodeId>,
        name: String,
    },
    CreateDir {
        node_id: NodeId,
        parent_id: Option<NodeId>,
        name: String,
    },
    RenameNode {
        node_id: NodeId,
        doc_id: Option<DocId>,
        new_name: String,
    },
    MoveNode {
        node_id: NodeId,
        doc_id: Option<DocId>,
        new_parent_id: Option<NodeId>,
    },
    DeleteNode {
        node_id: NodeId,
        doc_id: Option<DocId>,
    },
}

impl StructureOp {
    pub fn node_id(&self) -> NodeId {
        match self {
            Self::CreateFile { node_id, .. }
            | Self::CreateDir { node_id, .. }
            | Self::RenameNode { node_id, .. }
            | Self::MoveNode { node_id, .. }
            | Self::DeleteNode { node_id, .. } => *node_id,
        }
    }

    pub fn doc_id(&self) -> Option<DocId> {
        match self {
            Self::CreateFile { doc_id, .. } => Some(*doc_id),
            Self::CreateDir { .. } => None,
            Self::RenameNode { doc_id, .. }
            | Self::MoveNode { doc_id, .. }
            | Self::DeleteNode { doc_id, .. } => *doc_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LedgerEvent {
    Content(ContentOp),
    Structure(StructureOp),
}

impl LedgerEvent {
    pub fn content(op: ContentOp) -> Self {
        Self::Content(op)
    }

    pub fn structure(op: StructureOp) -> Self {
        Self::Structure(op)
    }

    pub fn content_op(&self) -> Option<&ContentOp> {
        match self {
            Self::Content(op) => Some(op),
            Self::Structure(_) => None,
        }
    }

    pub fn into_content_op(self) -> Option<ContentOp> {
        match self {
            Self::Content(op) => Some(op),
            Self::Structure(_) => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub doc_id: Option<DocId>,
    pub event: LedgerEvent,
    pub timestamp: i64,
    pub peer_id: PeerId,
    pub seq: u64,
    pub client_id: Option<u64>,
    pub client_op_id: Option<u64>,
}

impl LedgerEntry {
    pub fn new_content(
        doc_id: DocId,
        op: ContentOp,
        timestamp: i64,
        peer_id: PeerId,
        seq: u64,
        client_id: Option<u64>,
        client_op_id: Option<u64>,
    ) -> Self {
        Self {
            doc_id: Some(doc_id),
            event: LedgerEvent::content(op),
            timestamp,
            peer_id,
            seq,
            client_id,
            client_op_id,
        }
    }

    pub fn new_structure(op: StructureOp, timestamp: i64, peer_id: PeerId, seq: u64) -> Self {
        Self {
            doc_id: op.doc_id(),
            event: LedgerEvent::structure(op),
            timestamp,
            peer_id,
            seq,
            client_id: None,
            client_op_id: None,
        }
    }

    pub fn content_op(&self) -> Option<&ContentOp> {
        self.event.content_op()
    }

    pub fn cloned_content_op(&self) -> Option<ContentOp> {
        self.event.clone().into_content_op()
    }

    pub fn structure_node_id(&self) -> Option<NodeId> {
        match &self.event {
            LedgerEvent::Structure(op) => Some(op.node_id()),
            LedgerEvent::Content(_) => None,
        }
    }
}
