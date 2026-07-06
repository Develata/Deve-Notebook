use super::{endpoint, ready_probe};
use crate::native_adapter::{
    NativeAdapterPlatform, NativeAdapterSnapshot, NativeAdapterState, NativePlatformEventEffect,
    NativePlatformEventKind, NativeRuntimeReadiness, can_load_native_web_shell,
    can_show_native_writable_shell, classify_native_platform_event, platform_event_can_grant_write,
};

#[test]
fn writable_shell_requires_runtime_ready_writer_and_current_scope() {
    let snapshot = NativeAdapterSnapshot {
        platform: NativeAdapterPlatform::Desktop,
        state: NativeAdapterState::RuntimeReady,
        endpoint: Some(endpoint(
            "http://127.0.0.1:3001",
            "ws://127.0.0.1:3001",
            true,
        )),
        readiness: ready_probe(),
    };

    assert!(can_show_native_writable_shell(&snapshot));

    let stale_scope = NativeAdapterSnapshot {
        readiness: NativeRuntimeReadiness {
            scope_nonce_current: false,
            ..ready_probe()
        },
        ..snapshot.clone()
    };
    assert!(!can_show_native_writable_shell(&stale_scope));

    let missing_writer = NativeAdapterSnapshot {
        readiness: NativeRuntimeReadiness {
            writer_ready: false,
            ..ready_probe()
        },
        ..snapshot
    };
    assert!(!can_show_native_writable_shell(&missing_writer));
}

#[test]
fn native_reprobe_before_write_requires_full_runtime_readiness() {
    assert!(!ready_probe().needs_reprobe_before_write());

    for readiness in [
        NativeRuntimeReadiness {
            endpoint_reachable: false,
            ..ready_probe()
        },
        NativeRuntimeReadiness {
            node_role_readable: false,
            ..ready_probe()
        },
        NativeRuntimeReadiness {
            writer_ready: false,
            ..ready_probe()
        },
        NativeRuntimeReadiness {
            scope_nonce_current: false,
            ..ready_probe()
        },
    ] {
        assert!(readiness.needs_reprobe_before_write());
    }
}

#[test]
fn writable_shell_revalidates_injected_endpoint() {
    let snapshot = NativeAdapterSnapshot {
        platform: NativeAdapterPlatform::Desktop,
        state: NativeAdapterState::RuntimeReady,
        endpoint: Some(endpoint(
            "http://127.0.0.1.attacker.invalid:3001",
            "ws://127.0.0.1:3001",
            true,
        )),
        readiness: ready_probe(),
    };

    assert!(!can_load_native_web_shell(&snapshot));
    assert!(!can_show_native_writable_shell(&snapshot));
}

#[test]
fn session_invalid_and_service_offline_never_allow_writable_shell() {
    for state in [
        NativeAdapterState::SessionInvalid,
        NativeAdapterState::ServiceOffline,
        NativeAdapterState::ServiceRestarting,
    ] {
        let snapshot = NativeAdapterSnapshot {
            platform: NativeAdapterPlatform::Desktop,
            state,
            endpoint: Some(endpoint(
                "http://127.0.0.1:3001",
                "ws://127.0.0.1:3001",
                true,
            )),
            readiness: ready_probe(),
        };

        assert!(!can_show_native_writable_shell(&snapshot));
        assert!(snapshot.unauthorized_or_recovery_gate());
    }
}

#[test]
fn mobile_foreground_reprobe_does_not_restore_stale_write_scope() {
    let effect = classify_native_platform_event(
        NativeAdapterPlatform::Mobile,
        NativePlatformEventKind::Resumed,
    );
    let snapshot = NativeAdapterSnapshot {
        platform: NativeAdapterPlatform::Mobile,
        state: NativeAdapterState::ForegroundReprobe,
        endpoint: Some(endpoint(
            "http://127.0.0.1:3001",
            "ws://127.0.0.1:3001",
            true,
        )),
        readiness: NativeRuntimeReadiness {
            scope_nonce_current: false,
            ..ready_probe()
        },
    };

    assert_eq!(effect, NativePlatformEventEffect::RequireForegroundReprobe);
    assert!(snapshot.state.requires_fresh_handshake());
    assert!(!can_show_native_writable_shell(&snapshot));
}

#[test]
fn network_offline_is_a_hint_not_a_write_grant_or_revocation() {
    let effect = classify_native_platform_event(
        NativeAdapterPlatform::Desktop,
        NativePlatformEventKind::NetworkOffline,
    );
    let snapshot = NativeAdapterSnapshot {
        platform: NativeAdapterPlatform::Desktop,
        state: NativeAdapterState::RuntimeReady,
        endpoint: Some(endpoint(
            "http://127.0.0.1:3001",
            "ws://127.0.0.1:3001",
            true,
        )),
        readiness: ready_probe(),
    };

    assert_eq!(effect, NativePlatformEventEffect::NetworkHintOnly);
    assert!(!platform_event_can_grant_write(
        NativePlatformEventKind::NetworkOffline
    ));
    assert!(can_show_native_writable_shell(&snapshot));
}
