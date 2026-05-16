// crates\core\src\protocol
//! # WebSocket Protocol (通信协议)
//! plan_ref:
//!   - 05_network#server-ws-runtime
//!   - 05_network#web-ws-runtime
//!   - 16_web_thin_client_ledger#web-edit-intent
//!
//! **架构作用**:
//! 定义客户端与服务端之间的 WebSocket 通信消息格式。
//!
//! **核心功能清单**:
//! - `ClientMessage`: 定义客户端发起的请求（Edit, List, Open, Create, Copy, Move, Delete 等）。
//! - `ServerMessage`: 定义服务端推送的响应与事件（DocList, Snapshot, NewOps, ProtocolError 等）。
//! - `Op`: 定义 CRDT 操作单元。
//!
//! **类型**: Core MUST (核心必选)
//!
//! - `ServerMessage`: 服务端发送给客户端的消息
//!   - Ack（确认）, NewOp（新操作）, Snapshot（快照）
//!   - History（历史）, DocList（文档列表）, ProtocolError（结构化错误）

pub mod auth;
pub mod client;
mod client_scope;
pub mod confirmed_op;
pub mod doc_file_op_errors;
pub mod error;
pub mod frame;
mod json_wire;
pub mod merge_conflict;
pub mod sc_path_target;
pub mod scope_nonce;
pub mod server;
pub mod session_proof;
pub mod sync_push_header;

pub use client::ClientMessage;
pub use client_scope::ClientMessageScopeGate;
pub use confirmed_op::{ClientOrigin, ConfirmedOp};
pub use error::{ServerError, ServerErrorCode};
pub use frame::{MAX_WS_FRAME_BYTES, MIN_SUPPORTED_WS_PROTOCOL_VERSION, WS_PROTOCOL_VERSION};
pub use merge_conflict::{ConflictHunk, MergeConflictAction};
pub use sc_path_target::ScPathTarget;
pub use scope_nonce::{ScopeNonce, SwitchNonce};
pub use server::ServerMessage;
pub use session_proof::SessionProof;
pub use sync_push_header::{
    SyncPayloadKind, SyncPushHeader, SyncSourceProof, SyncSourceProofError,
};
