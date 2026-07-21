//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract

#[cfg(test)]
use deve_core::native_adapter::{
    NativeProcessAdapter, NativeServiceSupervisor, NativeServiceSupervisorSnapshot,
};
use deve_core::{
    config::RuntimeEnvironment, native_adapter::NativeEndpointReady, security::AuthConfig,
};
use std::{
    fmt,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};

use super::node_role::NativeServiceSummary;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerLaunchOptions {
    port: u16,
    bind_host: IpAddr,
    advertised_host: &'static str,
    runtime_environment: RuntimeEnvironment,
    native: Option<NativeLaunchSession>,
    repo_creation_projection_base: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeLaunchSession {
    session_bound: bool,
    auth_material: Option<NativeLoopbackAuthMaterial>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct NativeLoopbackAuthMaterial {
    session_bootstrap_secret: String,
    auth_secret: String,
    auth_user: String,
    auth_password_hash: String,
    allowed_origins: Vec<String>,
}

impl fmt::Debug for NativeLoopbackAuthMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeLoopbackAuthMaterial")
            .field("session_bootstrap_secret", &"<redacted>")
            .field("auth_secret", &"<redacted>")
            .field("auth_user", &self.auth_user)
            .field("auth_password_hash", &"<redacted>")
            .field("allowed_origins", &self.allowed_origins)
            .finish()
    }
}

impl NativeLoopbackAuthMaterial {
    // Used by the deve_cli library entrypoint consumed by native shells; the
    // deve_cli bin target compiles this module without native_runtime.
    #[allow(dead_code)]
    pub fn new(
        session_bootstrap_secret: impl Into<String>,
        auth_secret: impl Into<String>,
        auth_user: impl Into<String>,
        auth_password_hash: impl Into<String>,
        allowed_origins: Vec<String>,
    ) -> Self {
        Self {
            session_bootstrap_secret: session_bootstrap_secret.into(),
            auth_secret: auth_secret.into(),
            auth_user: auth_user.into(),
            auth_password_hash: auth_password_hash.into(),
            allowed_origins,
        }
    }

    pub fn session_bootstrap_secret(&self) -> &str {
        &self.session_bootstrap_secret
    }

    #[allow(dead_code)]
    pub fn auth_config(&self) -> anyhow::Result<AuthConfig> {
        AuthConfig::from_material(
            self.auth_secret.clone(),
            self.auth_user.clone(),
            self.auth_password_hash.clone(),
        )
    }

    #[allow(dead_code)]
    pub fn allowed_origins(&self) -> &[String] {
        &self.allowed_origins
    }
}

impl ServerLaunchOptions {
    pub fn release(port: u16) -> Self {
        Self {
            port,
            bind_host: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            advertised_host: "0.0.0.0",
            runtime_environment: RuntimeEnvironment::from_env(),
            native: None,
            repo_creation_projection_base: None,
        }
    }

    pub fn loopback_release(port: u16) -> Self {
        Self {
            port,
            bind_host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            advertised_host: "127.0.0.1",
            runtime_environment: RuntimeEnvironment::from_env(),
            native: None,
            repo_creation_projection_base: None,
        }
    }

    pub fn native_loopback(port: u16, session_bound: bool) -> Self {
        Self {
            port,
            bind_host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            advertised_host: "127.0.0.1",
            runtime_environment: RuntimeEnvironment::from_env(),
            native: Some(NativeLaunchSession {
                session_bound,
                auth_material: None,
            }),
            repo_creation_projection_base: None,
        }
    }

    #[allow(dead_code)]
    pub fn native_loopback_with_auth_material(
        port: u16,
        session_bound: bool,
        auth_material: NativeLoopbackAuthMaterial,
    ) -> Self {
        Self {
            port,
            bind_host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            advertised_host: "127.0.0.1",
            runtime_environment: RuntimeEnvironment::from_env(),
            native: Some(NativeLaunchSession {
                session_bound,
                auth_material: Some(auth_material),
            }),
            repo_creation_projection_base: None,
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn with_runtime_environment(mut self, environment: RuntimeEnvironment) -> Self {
        self.runtime_environment = environment;
        self
    }

    pub fn with_repo_creation_projection_base(mut self, base: Option<PathBuf>) -> Self {
        self.repo_creation_projection_base = base;
        self
    }

    pub fn repo_creation_projection_base(&self) -> Option<&Path> {
        self.repo_creation_projection_base.as_deref()
    }

    pub fn runtime_environment(&self) -> RuntimeEnvironment {
        self.runtime_environment
    }

    pub fn bind_addr(&self) -> SocketAddr {
        SocketAddr::new(self.bind_host, self.port)
    }

    pub fn ws_display_base(&self) -> String {
        format!("ws://{}:{}", self.advertised_host, self.port)
    }

    pub fn node_role_label(&self) -> &'static str {
        if self.native.is_some() {
            "native-main"
        } else {
            "main"
        }
    }

    pub fn native_service_summary(&self) -> Option<NativeServiceSummary> {
        let native = self.native.as_ref()?;
        let endpoint = self.native_endpoint(native.session_bound);
        Some(NativeServiceSummary {
            state: if native.session_bound {
                "endpoint_ready".into()
            } else {
                "session_pending".into()
            },
            endpoint: Some(endpoint),
            offline: None,
        })
    }

    pub fn is_native_loopback(&self) -> bool {
        self.native.is_some()
    }

    pub fn native_auth_material(&self) -> Option<&NativeLoopbackAuthMaterial> {
        self.native
            .as_ref()
            .and_then(|native| native.auth_material.as_ref())
    }

    pub fn native_allowed_origins(&self) -> Option<&[String]> {
        self.native_auth_material()
            .map(NativeLoopbackAuthMaterial::allowed_origins)
    }

    fn native_endpoint(&self, session_bound: bool) -> NativeEndpointReady {
        NativeEndpointReady {
            http_base: format!("http://{}:{}", self.advertised_host, self.port),
            ws_base: self.ws_display_base(),
            node_role: self.node_role_label().to_string(),
            session_bound,
        }
    }

    #[cfg(test)]
    fn native_supervisor_snapshot(&self) -> Option<NativeServiceSupervisorSnapshot> {
        let native = self.native.as_ref()?;
        let mut process = NativeProcessAdapter::default();
        let mut supervisor = NativeServiceSupervisor::new(2);
        supervisor.start();
        let endpoint_snapshot = process
            .bind_existing_endpoint(self.native_endpoint(false))
            .ok()?;
        supervisor.record_process_snapshot(&endpoint_snapshot);
        if native.session_bound {
            let session_snapshot = process.bind_session(true).ok()?;
            supervisor.record_process_snapshot(&session_snapshot);
        }
        Some(supervisor.snapshot())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deve_core::native_adapter::{
        NativeAdapterError, NativeServiceSupervisorState, validate_native_endpoint_bases,
        validate_native_endpoint_ready,
    };

    #[test]
    fn release_launch_keeps_public_bind_without_native_surface() {
        let launch = ServerLaunchOptions::release(3001);

        assert_eq!(launch.bind_addr(), SocketAddr::from(([0, 0, 0, 0], 3001)));
        assert_eq!(launch.node_role_label(), "main");
        assert_eq!(launch.runtime_environment(), RuntimeEnvironment::from_env());
        assert_eq!(launch.native_service_summary(), None);
    }

    #[test]
    fn loopback_release_is_main_without_native_session_surface() {
        let launch = ServerLaunchOptions::loopback_release(3001);

        assert_eq!(launch.bind_addr(), SocketAddr::from(([127, 0, 0, 1], 3001)));
        assert_eq!(launch.ws_display_base(), "ws://127.0.0.1:3001");
        assert_eq!(launch.node_role_label(), "main");
        assert_eq!(launch.native_service_summary(), None);
        assert!(!launch.is_native_loopback());
    }

    #[test]
    fn launch_can_carry_explicit_runtime_environment() {
        let launch = ServerLaunchOptions::release(3001)
            .with_runtime_environment(RuntimeEnvironment::Development);

        assert_eq!(
            launch.runtime_environment(),
            RuntimeEnvironment::Development
        );
        assert_eq!(launch.bind_addr(), SocketAddr::from(([0, 0, 0, 0], 3001)));
        assert_eq!(launch.native_service_summary(), None);
    }

    #[test]
    fn native_launch_binds_loopback_and_reports_endpoint() {
        let launch = ServerLaunchOptions::native_loopback(3001, true);
        let summary = launch
            .native_service_summary()
            .expect("native service summary");
        let endpoint = summary.endpoint.as_ref().expect("native endpoint");

        assert_eq!(launch.bind_addr(), SocketAddr::from(([127, 0, 0, 1], 3001)));
        assert_eq!(summary.state, "endpoint_ready");
        assert_eq!(endpoint.http_base, "http://127.0.0.1:3001");
        assert_eq!(endpoint.ws_base, "ws://127.0.0.1:3001");
        assert_eq!(validate_native_endpoint_ready(endpoint), Ok(()));
    }

    #[test]
    fn native_launch_can_report_session_pending_without_endpoint_scan() {
        let launch = ServerLaunchOptions::native_loopback(3001, false);
        let summary = launch
            .native_service_summary()
            .expect("native service summary");
        let endpoint = summary.endpoint.as_ref().expect("native endpoint");

        assert_eq!(summary.state, "session_pending");
        assert_eq!(validate_native_endpoint_bases(endpoint), Ok(()));
        assert_eq!(
            validate_native_endpoint_ready(endpoint),
            Err(NativeAdapterError::SessionNotBound)
        );
    }

    #[test]
    fn native_launch_can_carry_runtime_auth_material_without_debug_leak() {
        let session_secret = "session_secret_key_at_least_32_bytes".to_string();
        let auth_secret = "auth_secret_key_at_least_32_bytes!!".to_string();
        let password_hash = deve_core::security::auth::password::hash_password("runtime-password")
            .expect("password hash");
        let launch = ServerLaunchOptions::native_loopback_with_auth_material(
            3001,
            false,
            NativeLoopbackAuthMaterial::new(
                session_secret.clone(),
                auth_secret.clone(),
                "native",
                password_hash,
                vec!["http://tauri.localhost".to_string()],
            ),
        );

        let material = launch.native_auth_material().expect("auth material");
        assert_eq!(material.session_bootstrap_secret(), session_secret);
        assert_eq!(
            material.auth_config().expect("auth config").username,
            "native"
        );
        assert_eq!(
            launch.native_allowed_origins().expect("allowed origins"),
            ["http://tauri.localhost".to_string()].as_slice()
        );

        let debug = format!("{launch:?}");
        assert!(!debug.contains(&session_secret));
        assert!(!debug.contains(&auth_secret));
    }

    #[test]
    fn native_launch_supervisor_tracks_endpoint_and_session_boundaries() {
        let pending = ServerLaunchOptions::native_loopback(3001, false)
            .native_supervisor_snapshot()
            .expect("native supervisor");
        assert_eq!(pending.state, NativeServiceSupervisorState::EndpointHealthy);

        let ready = ServerLaunchOptions::native_loopback(3001, true)
            .native_supervisor_snapshot()
            .expect("native supervisor");
        assert_eq!(
            ready.state,
            NativeServiceSupervisorState::SessionHandoffReady
        );
    }

    #[test]
    fn release_launch_has_no_native_supervisor_surface() {
        assert_eq!(
            ServerLaunchOptions::release(3001).native_supervisor_snapshot(),
            None
        );
    }
}
