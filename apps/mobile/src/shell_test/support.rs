use crate::{MobileSessionMaterial, MobileShell};
use deve_core::native_adapter::{NativeEndpointReady, NativeRuntimeReadiness};

pub(crate) fn endpoint() -> NativeEndpointReady {
    NativeEndpointReady {
        http_base: "http://127.0.0.1:3001".to_string(),
        ws_base: "ws://127.0.0.1:3001".to_string(),
        node_role: "mobile-main".to_string(),
        session_bound: false,
    }
}

pub(crate) fn ready_probe() -> NativeRuntimeReadiness {
    NativeRuntimeReadiness {
        endpoint_reachable: true,
        auth_status_valid: true,
        node_role_readable: true,
        repo_handshake_complete: true,
        writer_ready: true,
        scope_nonce_current: true,
    }
}

pub(crate) fn bound_shell() -> MobileShell {
    let mut shell = MobileShell::new();
    shell.start_service();
    shell.bind_endpoint(endpoint()).expect("bind endpoint");
    shell
        .bind_session(MobileSessionMaterial::bound())
        .expect("bind session");
    shell
}
