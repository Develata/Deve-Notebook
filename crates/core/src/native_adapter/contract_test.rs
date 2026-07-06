mod backend_preference;
mod endpoint_validation;
mod readiness;
mod shell_mode;

use super::{NativeEndpointReady, NativeRuntimeReadiness};

fn endpoint(http_base: &str, ws_base: &str, session_bound: bool) -> NativeEndpointReady {
    NativeEndpointReady {
        http_base: http_base.to_string(),
        ws_base: ws_base.to_string(),
        node_role: "main".to_string(),
        session_bound,
    }
}

fn ready_probe() -> NativeRuntimeReadiness {
    NativeRuntimeReadiness {
        endpoint_reachable: true,
        auth_status_valid: true,
        node_role_readable: true,
        repo_handshake_complete: true,
        writer_ready: true,
        scope_nonce_current: true,
    }
}
