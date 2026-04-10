// apps/web/src/api/mod.rs
//! # WebSocket API 模块
//!
//! 本模块提供 `WsService` 用于与后端进行 WebSocket 通信。

mod ai_backend;
mod auth_probe;
mod backoff;
mod connection;
mod connection_role;
mod connection_urls;
mod incoming;
mod output;
mod service;
mod socket;
mod status;
mod writer_id;

pub use self::ai_backend::{AiBackendCapabilities, fetch_ai_backend_capabilities};
pub use self::auth_probe::{AuthProbe, probe_auth_status};
pub use self::service::WsService;
pub use self::status::ConnectionStatus;
