use super::support::{bound_shell, ready_probe};
use crate::{MobileLifecycleEvent, MobileServiceState, MobileShellError};
use deve_core::native_adapter::{NativePlatformEventKind, NativeRuntimeReadiness};

#[test]
fn mobile_background_resume_requires_fresh_reprobe_before_write() {
    let mut shell = bound_shell();
    assert!(shell.mark_runtime_ready(ready_probe()));

    let background = shell.handle_lifecycle_event(MobileLifecycleEvent::Background);
    assert_eq!(background, NativePlatformEventKind::Background);
    assert_eq!(
        shell.snapshot().state,
        MobileServiceState::BackgroundSuspended
    );
    assert!(matches!(
        shell.bootstrap_for_web(),
        Err(MobileShellError::ForegroundReprobeRequired)
    ));

    let resumed = shell.handle_lifecycle_event(MobileLifecycleEvent::Resumed);
    assert_eq!(resumed, NativePlatformEventKind::Resumed);
    let snapshot = shell.snapshot();
    assert_eq!(snapshot.state, MobileServiceState::ForegroundReprobe);
    assert!(!snapshot.readiness.auth_status_valid);
    assert!(!snapshot.readiness.node_role_readable);
    assert!(!snapshot.readiness.repo_handshake_complete);
    assert!(!snapshot.readiness.writer_ready);
    assert!(!snapshot.readiness.scope_nonce_current);
    assert_eq!(
        shell
            .recovery_bootstrap_for_web()
            .expect("recovery bootstrap")
            .service_state,
        "foreground_reprobe"
    );
}

#[test]
fn mobile_reprobe_does_not_restore_write_without_current_scope_nonce() {
    let mut shell = bound_shell();
    shell.handle_lifecycle_event(MobileLifecycleEvent::Resumed);
    let missing_node_role = NativeRuntimeReadiness {
        node_role_readable: false,
        ..ready_probe()
    };
    assert!(!shell.complete_foreground_reprobe(missing_node_role));
    assert_eq!(
        shell.snapshot().state,
        MobileServiceState::ForegroundReprobe
    );

    let missing_writer = NativeRuntimeReadiness {
        writer_ready: false,
        ..ready_probe()
    };
    assert!(!shell.complete_foreground_reprobe(missing_writer));
    assert_eq!(
        shell.snapshot().state,
        MobileServiceState::ForegroundReprobe
    );

    let ready_without_scope = NativeRuntimeReadiness {
        scope_nonce_current: false,
        ..ready_probe()
    };

    assert!(!shell.complete_foreground_reprobe(ready_without_scope));
    assert_eq!(
        shell.snapshot().state,
        MobileServiceState::ForegroundReprobe
    );
    assert!(shell.complete_foreground_reprobe(ready_probe()));
    assert_eq!(shell.snapshot().state, MobileServiceState::RuntimeReady);
}

#[test]
fn mobile_recovery_bootstrap_for_reprobe_exposes_only_structured_state() {
    let mut background = bound_shell();
    assert!(background.mark_runtime_ready(ready_probe()));
    background.handle_lifecycle_event(MobileLifecycleEvent::Background);
    assert_eq!(
        background.snapshot().state,
        MobileServiceState::BackgroundSuspended
    );
    assert_reprobe_recovery_bootstrap_is_minimal(&background);

    let mut foreground = bound_shell();
    foreground.handle_lifecycle_event(MobileLifecycleEvent::Resumed);
    assert_reprobe_recovery_bootstrap_is_minimal(&foreground);
}

#[test]
fn mobile_network_events_are_hints_not_write_grants() {
    let mut shell = bound_shell();
    let event = shell.handle_lifecycle_event(MobileLifecycleEvent::NetworkOffline);

    assert_eq!(event, NativePlatformEventKind::NetworkOffline);
    assert_eq!(shell.snapshot().state, MobileServiceState::SessionBound);
    assert!(!shell.snapshot().readiness.writer_ready);
}

fn assert_reprobe_recovery_bootstrap_is_minimal(shell: &crate::MobileShell) {
    let script = shell
        .recovery_bootstrap_for_web()
        .expect("reprobe recovery bootstrap")
        .script_tag()
        .expect("reprobe recovery script");

    assert!(script.contains("\"service_state\":\"foreground_reprobe\""));
    for forbidden in [
        "http_base",
        "ws_base",
        "node_role",
        "session_bound",
        "scope_nonce",
        "writer",
        "token",
        "secret",
        "http://127.0.0.1",
        "ws://127.0.0.1",
    ] {
        assert!(
            !script.contains(forbidden),
            "recovery bootstrap leaked {forbidden}: {script}"
        );
    }
}
