//! # 账本事件模型
//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!
//! Invariants:
//! - `ContentOp` 只描述文本内容变化。
//! - `StructureOp` 只描述节点/路径结构变化。
//! - `LedgerEntry` 的权威载荷永远是 `LedgerEvent`，而不是分散副作用。

use super::{DocId, FactActor, NodeId, PeerFactSeq, PeerId};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentOp {
    Insert { pos: u32, content: SmolStr },
    Delete { pos: u32, len: u32 },
}

pub type Op = ContentOp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeResolution {
    EstablishEqual,
    Auto,
    AcceptCurrent,
    AcceptIncoming,
    AcceptBoth,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeAnchor {
    pub source_peer_id: PeerId,
    pub source_waterline: PeerFactSeq,
    pub local_pre_merge_waterline: PeerFactSeq,
    pub source_state_hash: [u8; 32],
    pub result_hash: [u8; 32],
    pub resolution: MergeResolution,
}

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
    MergeAnchor(MergeAnchor),
}

impl LedgerEvent {
    pub fn content(op: ContentOp) -> Self {
        Self::Content(op)
    }

    pub fn structure(op: StructureOp) -> Self {
        Self::Structure(op)
    }

    pub fn merge_anchor(anchor: MergeAnchor) -> Self {
        Self::MergeAnchor(anchor)
    }

    pub fn content_op(&self) -> Option<&ContentOp> {
        match self {
            Self::Content(op) => Some(op),
            Self::Structure(_) | Self::MergeAnchor(_) => None,
        }
    }

    pub fn into_content_op(self) -> Option<ContentOp> {
        match self {
            Self::Content(op) => Some(op),
            Self::Structure(_) | Self::MergeAnchor(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub doc_id: Option<DocId>,
    pub event: LedgerEvent,
    pub timestamp: i64,
    pub origin_peer_id: PeerId,
    pub peer_seq: PeerFactSeq,
    pub actor: FactActor,
    pub client_id: Option<u64>,
    pub client_op_id: Option<u64>,
}

impl LedgerEntry {
    pub fn new_content(
        doc_id: DocId,
        op: ContentOp,
        timestamp: i64,
        origin_peer_id: PeerId,
        seq: u64,
        client_id: Option<u64>,
        client_op_id: Option<u64>,
    ) -> Self {
        Self::new_content_with_actor(
            doc_id,
            op,
            timestamp,
            origin_peer_id,
            PeerFactSeq::new(seq),
            FactActor::system(),
            client_id,
            client_op_id,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_content_with_actor(
        doc_id: DocId,
        op: ContentOp,
        timestamp: i64,
        origin_peer_id: PeerId,
        peer_seq: PeerFactSeq,
        actor: FactActor,
        client_id: Option<u64>,
        client_op_id: Option<u64>,
    ) -> Self {
        Self {
            doc_id: Some(doc_id),
            event: LedgerEvent::content(op),
            timestamp,
            origin_peer_id,
            peer_seq,
            actor,
            client_id,
            client_op_id,
        }
    }

    pub fn new_structure(
        op: StructureOp,
        timestamp: i64,
        origin_peer_id: PeerId,
        seq: u64,
    ) -> Self {
        Self::new_structure_with_actor(
            op,
            timestamp,
            origin_peer_id,
            PeerFactSeq::new(seq),
            FactActor::system(),
        )
    }

    pub fn new_structure_with_actor(
        op: StructureOp,
        timestamp: i64,
        origin_peer_id: PeerId,
        peer_seq: PeerFactSeq,
        actor: FactActor,
    ) -> Self {
        Self {
            doc_id: op.doc_id(),
            event: LedgerEvent::structure(op),
            timestamp,
            origin_peer_id,
            peer_seq,
            actor,
            client_id: None,
            client_op_id: None,
        }
    }

    pub fn new_merge_anchor_with_actor(
        doc_id: DocId,
        anchor: MergeAnchor,
        timestamp: i64,
        origin_peer_id: PeerId,
        peer_seq: PeerFactSeq,
        actor: FactActor,
    ) -> Self {
        Self {
            doc_id: Some(doc_id),
            event: LedgerEvent::merge_anchor(anchor),
            timestamp,
            origin_peer_id,
            peer_seq,
            actor,
            client_id: None,
            client_op_id: None,
        }
    }

    pub fn merge_anchor(&self) -> Option<&MergeAnchor> {
        match &self.event {
            LedgerEvent::MergeAnchor(anchor) => Some(anchor),
            LedgerEvent::Content(_) | LedgerEvent::Structure(_) => None,
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
            LedgerEvent::Content(_) | LedgerEvent::MergeAnchor(_) => None,
        }
    }
}
