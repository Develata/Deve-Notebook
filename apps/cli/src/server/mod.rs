// apps\cli\src\server
//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! Deve-Note 后端 WebSocket/HTTP 服务器模块边界。

pub mod agent_bridge;
pub mod ai_chat;
pub mod auth;
pub mod channel;
#[cfg(test)]
include!("test_modules.rs");
mod diff_projection;
mod error_classify;
pub mod handlers;
mod launch;
pub mod metrics;
pub mod node_role;
pub mod node_role_http;
mod notegit;
mod p2p;
mod p2p_connector;
mod p2p_status;
pub mod plugin_host;
pub mod plugin_response;
pub mod prewarm;
mod rate_limit;
mod repo_mutation;
mod repo_scope;
mod router;
mod runtime;
pub mod security;
pub mod session;
mod setup;
mod shadow_scope;
mod source_control_grants;
pub mod source_control_proxy;
mod start;
mod state;
pub mod static_files;
mod static_files_embed;
mod tree_state;
pub mod ws;

#[allow(unused_imports)]
pub use launch::NativeLoopbackAuthMaterial;
pub use launch::ServerLaunchOptions;
pub(crate) use start::{EmbeddedServerRuntime, ServerTransportRuntime, ServerTransportServeError};
#[allow(unused_imports)]
pub use start::{start_plugin_host_only, start_server_with_bound_listener};
pub use state::AppState;
