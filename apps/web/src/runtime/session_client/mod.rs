//! plan_ref:
//!   - 08_auth#mode-aware-logout-projection
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! Browser session client runtime.
//!
//! This is a Flow Coordination adapter for transport/session readiness. It
//! does not store business truth or perform authority writes.

use crate::api::{ConnectionStatus, WsService, current_bundled_local_backend};
use leptos::prelude::*;

#[allow(dead_code)]
#[derive(Clone)]
pub struct SessionClient {
    pub ws: WsService,
    pub connection_status: ReadSignal<ConnectionStatus>,
    pub status_text: Signal<String>,
    pub sync_banner: Signal<Option<String>>,
    pub set_sync_banner: WriteSignal<Option<String>>,
    pub handshake_ready: ReadSignal<bool>,
    pub handshake_scope_nonce: ReadSignal<Option<u64>>,
    pub on_retry_peer_registration: Callback<()>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SessionPresentationPolicy {
    BrowserSession,
    BundledLocalBackend,
}

impl SessionPresentationPolicy {
    pub(crate) fn current() -> Self {
        Self::from_bundled_local_backend(current_bundled_local_backend())
    }

    const fn from_bundled_local_backend(bundled_local_backend: bool) -> Self {
        if bundled_local_backend {
            Self::BundledLocalBackend
        } else {
            Self::BrowserSession
        }
    }

    pub(crate) const fn show_logout(self) -> bool {
        matches!(self, Self::BrowserSession)
    }
}

#[cfg(test)]
mod tests {
    use super::SessionPresentationPolicy;

    #[test]
    fn logout_projection_hides_only_bundled_local_backend_logout() {
        assert!(SessionPresentationPolicy::from_bundled_local_backend(false).show_logout());
        assert!(!SessionPresentationPolicy::from_bundled_local_backend(true).show_logout());
    }
}
