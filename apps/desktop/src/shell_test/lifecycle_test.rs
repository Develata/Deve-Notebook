use super::support::{bound_shell, ready_probe};
use crate::{DesktopServiceState, DesktopShellError};
use deve_core::native_adapter::{
    NativePlatformEventEffect, NativePlatformEventKind, NativeRuntimeReadiness,
};

#[test]
fn desktop_runtime_ready_requires_writer_and_current_scope() {
    let mut shell = bound_shell();

    let missing_node_role = NativeRuntimeReadiness {
        node_role_readable: false,
        ..ready_probe()
    };
    assert!(!shell.mark_runtime_ready(missing_node_role));
    assert_eq!(shell.snapshot().state, DesktopServiceState::SessionBound);

    let missing_writer = NativeRuntimeReadiness {
        writer_ready: false,
        ..ready_probe()
    };
    assert!(!shell.mark_runtime_ready(missing_writer));
    assert_eq!(shell.snapshot().state, DesktopServiceState::SessionBound);

    let stale_scope = NativeRuntimeReadiness {
        scope_nonce_current: false,
        ..ready_probe()
    };
    assert!(!shell.mark_runtime_ready(stale_scope));
    assert_eq!(shell.snapshot().state, DesktopServiceState::SessionBound);

    assert!(shell.mark_runtime_ready(ready_probe()));
    assert_eq!(shell.snapshot().state, DesktopServiceState::RuntimeReady);
}

#[test]
fn desktop_foreground_resume_requires_fresh_reprobe_before_write() {
    let mut shell = bound_shell();
    assert!(shell.mark_runtime_ready(ready_probe()));

    let effect = shell.handle_platform_event(NativePlatformEventKind::Resumed);
    assert_eq!(effect, NativePlatformEventEffect::RequireForegroundReprobe);
    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, DesktopServiceState::ForegroundReprobe);
    assert!(!snapshot.readiness.auth_status_valid);
    assert!(!snapshot.readiness.node_role_readable);
    assert!(!snapshot.readiness.repo_handshake_complete);
    assert!(!snapshot.readiness.writer_ready);
    assert!(!snapshot.readiness.scope_nonce_current);
    assert!(matches!(
        shell.bootstrap_for_web(),
        Err(DesktopShellError::ForegroundReprobeRequired)
    ));
    assert_eq!(
        shell
            .recovery_bootstrap_for_web()
            .expect("recovery bootstrap")
            .service_state,
        "foreground_reprobe"
    );
}

#[test]
fn desktop_foreground_reprobe_does_not_restore_stale_scope() {
    let mut shell = bound_shell();
    assert!(shell.mark_runtime_ready(ready_probe()));
    shell.handle_platform_event(NativePlatformEventKind::Foreground);

    let stale_scope = NativeRuntimeReadiness {
        scope_nonce_current: false,
        ..ready_probe()
    };
    assert!(!shell.complete_foreground_reprobe(stale_scope));
    assert_eq!(
        shell.snapshot().state,
        DesktopServiceState::ForegroundReprobe
    );
    assert!(shell.complete_foreground_reprobe(ready_probe()));
    assert_eq!(shell.snapshot().state, DesktopServiceState::RuntimeReady);
}

#[test]
fn desktop_network_events_are_hints_not_write_grants() {
    let mut shell = bound_shell();
    let effect = shell.handle_platform_event(NativePlatformEventKind::NetworkOffline);

    assert_eq!(effect, NativePlatformEventEffect::NetworkHintOnly);
    assert_eq!(shell.snapshot().state, DesktopServiceState::SessionBound);
    assert!(!shell.snapshot().readiness.writer_ready);
}
