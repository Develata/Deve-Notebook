// crates\core\src\protocol
//! # WebSocket Protocol (通信协议)
//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 07_network#web-ws-runtime
//!   - 07_network#projection-recovery-contract
//!   - 07_network#relay-proxy-attribution-contract
//!   - 09_web_thin_client_ledger#web-edit-intent
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
pub mod document_create;
pub mod error;
pub mod frame;
mod json_wire;
pub mod merge_conflict;
pub mod projection_recovery;
pub mod relay_proxy;
pub mod remote_import;
pub mod remote_projection;
pub mod repo_control;
pub mod sc_path_target;
pub mod scope_nonce;
pub mod server;
pub mod session_proof;
pub mod sync_push_header;

pub use client::ClientMessage;
pub use client_scope::ClientMessageScopeGate;
pub use confirmed_op::{ClientOrigin, ConfirmedOp};
pub use document_create::{
    DocumentCreateProjectionOutcome, DocumentCreateRequest, DocumentCreateResponse,
    DocumentCreateResponseContext,
};
pub use error::{ServerError, ServerErrorCode};
pub use frame::{
    MAX_SYNC_FACT_BYTES_PER_PAYLOAD, MAX_SYNC_FACTS_PER_PAYLOAD, MAX_WS_FRAME_BYTES,
    MIN_SUPPORTED_WS_PROTOCOL_VERSION, WS_PROTOCOL_VERSION, server_binary_payload_size,
};
pub use merge_conflict::{ConflictHunk, MergeConflictAction};
pub use projection_recovery::{
    DocumentRecoveryScope, ProjectionRecoveryCause, ProjectionRecoveryPlan,
    ProjectionRecoveryRequired,
};
pub use relay_proxy::{
    DirectSyncPushAttributionInput, DirectSyncSnapshotAttributionInput, RelayProxyRoute,
    RelayProxyRouteError, RelayProxyRouteInput, RelayProxySnapshotRouteInput,
    SourceProofRequirement, SyncAttributionError, SyncPushAttributionInput,
    SyncSnapshotAttributionInput, plan_relay_proxy_route, plan_relay_proxy_snapshot_route,
    validate_direct_sync_push_attribution, validate_direct_sync_snapshot_attribution,
    validate_sync_push_attribution, validate_sync_snapshot_attribution,
};
pub use remote_import::{
    REMOTE_IMPORT_DEFAULT_PAGE_SIZE, REMOTE_IMPORT_MAX_PAGE_SIZE, RemoteImportApplyReceipt,
    RemoteImportBlocker, RemoteImportCandidatePage, RemoteImportCandidateRevision,
    RemoteImportCandidateView, RemoteImportChangeKind, RemoteImportEntryId, RemoteImportPageCursor,
    RemoteImportProjectionOutcome, RemoteImportRequest, RemoteImportRequestContext,
    RemoteImportResponse, RemoteImportResponseContext, RemoteImportSessionId,
    RemoteImportSessionView, RemoteImportState, RemoteProjectionPushRequest,
    RemoteProjectionPushResponse,
};
pub use remote_projection::RemoteProjectionProvider;
pub use repo_control::{
    LocalRepoRemovalBlocker, LocalRepoRemovalDeletedCategory, LocalRepoRemovalPreservedCategory,
    LocalRepoRemovalPreview, LocalRepoRemovalWarning, OpaqueFallbackBinding,
    RemovalConfirmationToken, RepoAliasBinding, RepoControlRequest, RepoControlResponse,
    RepoLifecycleIntent, RepoLifecycleOperation, RepoLifecycleOutcome, RepoLifecycleState,
    RepoReadiness, RepoRemovalFinalScope,
};
pub use sc_path_target::ScPathTarget;
pub use scope_nonce::{ScopeNonce, SwitchNonce};
pub use server::{RepoListEntry, ServerMessage};
pub use session_proof::SessionProof;
pub use sync_push_header::{
    SyncPayloadKind, SyncPushHeader, SyncSourceProof, SyncSourceProofError,
};
