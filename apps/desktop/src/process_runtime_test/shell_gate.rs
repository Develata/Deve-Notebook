use deve_core::native_adapter::{NativeProcessRuntimeState, NativeRuntimeReadiness};

use crate::process_runtime::DesktopLocalServiceRuntime;
use crate::{DesktopServiceState, DesktopSessionMaterial, DesktopShell};

use super::support::{
    RecordingLauncher, enabled_policy, endpoint, handle, healthy_probe, valid_spawn_spec,
};

#[test]
fn desktop_local_service_runtime_does_not_unlock_writable_shell_without_writer_gate() {
    let launcher = RecordingLauncher::with_handle(handle());
    let mut runtime = DesktopLocalServiceRuntime::with_launcher(enabled_policy(), 1, launcher);
    runtime
        .start(&valid_spawn_spec(), 1)
        .expect("request start");
    runtime.record_endpoint_probe(endpoint(false), healthy_probe(), 2);
    runtime.record_session_handoff(true, 3);
    runtime.mark_runtime_ready(4);
    assert_eq!(
        runtime.snapshot().state,
        NativeProcessRuntimeState::RuntimeReady
    );

    let mut shell = DesktopShell::new();
    shell.start_service();
    shell.bind_endpoint(endpoint(false)).expect("bind endpoint");
    shell
        .bind_session(DesktopSessionMaterial::bound())
        .expect("bind session");

    let readiness = NativeRuntimeReadiness {
        endpoint_reachable: true,
        auth_status_valid: true,
        node_role_readable: true,
        repo_handshake_complete: true,
        writer_ready: false,
        scope_nonce_current: true,
    };

    assert!(!shell.mark_runtime_ready(readiness));
    assert_eq!(shell.snapshot().state, DesktopServiceState::SessionBound);
}
