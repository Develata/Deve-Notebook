//! # WebSocket Protocol (通信协议)
//!
//! **架构作用**:
//! 定义客户端与服务端之间的 WebSocket 通信消息格式。
//!
//! **核心功能清单**:
//! - `ClientMessage`: 定义客户端发起的请求（Edit, List, Open, Create, Copy, Move, Delete 等）。
//! - `ServerMessage`: 定义服务端推送的响应与事件（DocList, Snapshot, NewOps, Error 等）。
//! - `Op`: 定义 CRDT 操作单元。
//!
//! **类型**: Core MUST (核心必选)
//!
//! - `ServerMessage`: 服务端发送给客户端的消息
//!   - Ack（确认）, NewOp（新操作）, Snapshot（快照）
//!   - History（历史）, DocList（文档列表）, Error（错误）

use serde::{Serialize, Deserialize};
use crate::models::{DocId, Op, PeerId, LedgerEntry, VersionVector};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    /// P2P Handshake Request
    SyncHello {
        peer_id: PeerId,
        vector: VersionVector,
    },
    /// Request missing operations
    SyncRequest {
        /// List of (PeerId, Range Start, Range End)
        /// Note: Range<u64> is not serializable by default, so we use (u64, u64)
        requests: Vec<(PeerId, (u64, u64))>,
    },
    /// Push operations to peer
    SyncPush {
        ops: Vec<LedgerEntry>,
    },
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
    /// Copy a document to a new location
    CopyDoc {
        src_path: String,
        dest_path: String,
    },
    /// Move a document to a new location
    MoveDoc {
        src_path: String,
        dest_path: String,
    },
    /// Call a plugin function
    PluginCall {
        req_id: String,
        plugin_id: String,
        fn_name: String,
        args: Vec<serde_json::Value>,
    },
    /// Full-text search query
    Search {
        query: String,
        limit: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Server acknowledges that an Op has been persisted with a specific Sequence Number.
    Ack {
        doc_id: DocId,
        seq: u64,
    },
    /// P2P Handshake Response (Offer what you are missing)
    SyncOffer {
        /// List of (PeerId, Range Start, Range End)
        missing: Vec<(PeerId, (u64, u64))>,
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
    /// Response from a plugin call
    PluginResponse {
        req_id: String,
        result: Option<serde_json::Value>,
        error: Option<String>,
    },
    /// Full-text search results
    SearchResults {
        results: Vec<(String, String, f32)>, // (doc_id as UUID string, path, score)
    },
    /// Error message
    Error(String),
}
