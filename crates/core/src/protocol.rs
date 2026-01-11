//! # WebSocket 协议定义
//!
//! 本模块定义了客户端（Web 前端）与服务端（CLI 后端）之间
//! WebSocket 通信使用的消息类型。
//!
//! ## 消息类型
//!
//! - `ClientMessage`: 客户端发送给服务端的消息
//!   - Edit（编辑）, RequestHistory（请求历史）, ListDocs（列表文档）
//!   - OpenDoc（打开文档）, CreateDoc（创建文档）, RenameDoc（重命名）, DeleteDoc（删除）
//!
//! - `ServerMessage`: 服务端发送给客户端的消息
//!   - Ack（确认）, NewOp（新操作）, Snapshot（快照）
//!   - History（历史）, DocList（文档列表）, Error（错误）

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
