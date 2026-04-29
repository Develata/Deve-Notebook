// apps/web/src/api/mod.rs
//! plan_ref:
//!   - 05_network#web-ws-runtime
//!
//! # WebSocket API 模块
//!
//! 本模块提供 `WsService` 用于与后端进行 WebSocket 通信。

mod ai_backend;
mod auth_probe;
mod backoff;
mod connection;
mod connection_role;
mod connection_urls;
mod git_mirror;
mod graph;
mod incoming;
mod native_bootstrap;
mod output;
mod service;
mod socket;
mod status;
mod writer_id;

pub use self::ai_backend::{AiBackendCapabilities, fetch_ai_backend_capabilities};
pub use self::auth_probe::{AuthProbe, probe_auth_status};
pub use self::git_mirror::fetch_git_mirror_repair_review;
pub use self::graph::fetch_graph_projection;
pub use self::service::WsService;
pub use self::status::ConnectionStatus;
