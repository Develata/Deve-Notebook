use serde::{Serialize, Deserialize};
use crate::models::{DocId, Op};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Client sends an edit operation for a specific document.
    Edit {
        doc_id: DocId,
        op: Op,
        client_id: u64,
    },
    /// Request full operation history for a document.
    RequestHistory {
        doc_id: DocId,
    },
    /// Request a list of all known documents.
    ListDocs,
    /// Request to open a specific document (get Snapshot).
    OpenDoc {
        doc_id: DocId,
    },
    /// Request to create a new document.
    CreateDoc {
        name: String,
    },
    RenameDoc {
        old_path: String,
        new_path: String,
    },
    DeleteDoc {
        path: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Server acknowledges that an Op has been persisted with a specific Sequence Number.
    Ack {
        doc_id: DocId,
        seq: u64,
    },
    /// Server broadcasts a new Op from another client.
    NewOp {
        doc_id: DocId,
        op: Op,
        seq: u64,
        client_id: u64,
    },
    /// Server sends the full content of the document (Initial Load).
    Snapshot {
        doc_id: DocId,
        content: String,
        version: u64,
    },
    /// Server sends the full history of operations (for Playback).
    History {
        doc_id: DocId,
        ops: Vec<(u64, Op)>,
    },
    /// Server sends list of documents.
    DocList {
        docs: Vec<(DocId, String)>,
    },
    /// Error message
    Error(String),
}
