//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract

use deve_core::native_adapter::NativeEndpointReady;
#[cfg(test)]
use deve_core::native_adapter::{
    NativeProcessAdapter, NativeServiceSupervisor, NativeServiceSupervisorSnapshot,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use super::node_role::NativeServiceSummary;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerLaunchOptions {
    port: u16,
    bind_host: IpAddr,
    advertised_host: &'static str,
    native: Option<NativeLaunchSession>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeLaunchSession {
    session_bound: bool,
}

impl ServerLaunchOptions {
    pub fn release(port: u16) -> Self {
        Self {
            port,
            bind_host: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            advertised_host: "0.0.0.0",
            native: None,
        }
    }

    pub fn native_loopback(port: u16, session_bound: bool) -> Self {
        Self {
            port,
            bind_host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            advertised_host: "127.0.0.1",
            native: Some(NativeLaunchSession { session_bound }),
        }
    }

    pub fn port(&self) -> u16 {
        self.port
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
