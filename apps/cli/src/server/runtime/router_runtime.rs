//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 11_ui_design/01_web#single-binary-distribution
//!
//! Static-file admission and Axum router assembly.

use crate::server::{AppState, router, static_files};
use axum::Router;
use deve_core::config::RuntimeEnvironment;
use std::sync::Arc;

use super::auth_runtime::AuthRuntimeParts;

pub(crate) fn build_runtime_router(
    app_state: Arc<AppState>,
    port: u16,
    auth: AuthRuntimeParts,
    p2p_inbound_token_env: Option<String>,
    runtime_environment: RuntimeEnvironment,
    allowed_origins_override: Option<&[String]>,
    ws_transport_runtime: Arc<crate::server::ws::transport::WsTransportRuntime>,
) -> anyhow::Result<Router> {
    static_files::validate_static_dir_override()?;
    match auth.native_session_bridge {
        Some(bridge) => router::build_app_with_native_session_and_p2p(
            app_state,
            port,
            auth.auth_config,
            Some(bridge),
            runtime_environment,
            allowed_origins_override,
            router::WsTransportRouterParts::new(p2p_inbound_token_env, ws_transport_runtime),
        ),
        None => router::build_app_with_native_session_and_p2p(
            app_state,
            port,
            auth.auth_config,
            None,
            runtime_environment,
            allowed_origins_override,
            router::WsTransportRouterParts::new(p2p_inbound_token_env, ws_transport_runtime),
        ),
    }
}
