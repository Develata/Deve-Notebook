use serde::{Serialize, Deserialize};
use crate::models::{DocId, Op};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Client sends an edit operation for a specific document.
    Edit {
        doc_id: DocId,
        op: Op,
    },
    // Future: Subscribe / Unsubscribe
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Server acknowledges that an Op has been persisted with a specific Sequence Number.
    Ack {
        doc_id: DocId,
        seq: u64,
    },
    /// Server broadcasts new Ops from other clients.
    NewOps {
        doc_id: DocId,
        ops: Vec<Op>,
    },
    /// Error message
    Error(String),
}
