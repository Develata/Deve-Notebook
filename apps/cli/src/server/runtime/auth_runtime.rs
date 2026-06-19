//! plan_ref:
//!   - 08_auth#auth-http-endpoints
//!   - 07_network#server-ws-runtime
//!
//! Auth and native-session runtime assembly.

use crate::server::{auth, launch::ServerLaunchOptions, router};
use deve_core::security::AuthConfig;
use std::sync::Arc;

pub(crate) struct AuthRuntimeParts {
    pub auth_config: Arc<AuthConfig>,
    pub native_session_bridge: Option<Arc<auth::handlers::NativeSessionBridge>>,
}

pub(crate) fn init_auth_runtime(launch: &ServerLaunchOptions) -> anyhow::Result<AuthRuntimeParts> {
    let auth_config = Arc::new(match launch.native_auth_material() {
        Some(material) => material.auth_config()?,
        None => router::load_auth_config(),
    });
    let native_session_bridge = match launch.native_auth_material() {
        Some(material) => Some(Arc::new(auth::handlers::NativeSessionBridge::new(
            material.session_bootstrap_secret().to_string(),
        )?)),
        None => auth::handlers::NativeSessionBridge::from_env(launch.is_native_loopback())
            .map(|bridge| bridge.map(Arc::new))?,
    };
    Ok(AuthRuntimeParts {
        auth_config,
        native_session_bridge,
    })
}
