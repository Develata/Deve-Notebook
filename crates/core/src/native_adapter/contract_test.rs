use super::*;

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

#[test]
fn native_endpoint_validation_accepts_loopback_bases() {
    let endpoint = endpoint("http://127.0.0.1:3001", "ws://localhost:3001", true);

    assert_eq!(validate_native_endpoint_ready(&endpoint), Ok(()));
}

#[test]
fn native_endpoint_validation_rejects_non_loopback_hosts() {
    let endpoint = endpoint("http://192.168.1.10:3001", "ws://127.0.0.1:3001", true);

    assert!(matches!(
        validate_native_endpoint_ready(&endpoint),
        Err(NativeAdapterError::NonLoopbackHost { field: "http_base" })
    ));
}

#[test]
fn native_endpoint_validation_rejects_scan_like_host_suffixes() {
    let endpoint = endpoint(
        "http://127.0.0.1.evil.example:3001",
        "ws://127.0.0.1:3001",
        true,
    );

    assert!(matches!(
        validate_native_endpoint_ready(&endpoint),
        Err(NativeAdapterError::NonLoopbackHost { field: "http_base" })
    ));
}

#[test]
fn native_endpoint_validation_rejects_url_credentials() {
    let endpoint = endpoint("http://token@127.0.0.1:3001", "ws://127.0.0.1:3001", true);

    assert!(matches!(
        validate_native_endpoint_ready(&endpoint),
        Err(NativeAdapterError::UserInfoForbidden { field: "http_base" })
    ));
}

#[test]
fn native_endpoint_validation_rejects_invalid_or_zero_ports() {
    for port in ["0", "65536", "not-a-port", ""] {
        let endpoint = endpoint(
            &format!("http://127.0.0.1:{port}"),
            "ws://127.0.0.1:3001",
            true,
        );

        assert!(matches!(
            validate_native_endpoint_ready(&endpoint),
            Err(NativeAdapterError::InvalidPort { field: "http_base" })
        ));
    }
}

#[test]
fn native_endpoint_ready_requires_session_binding() {
    let endpoint = endpoint("http://127.0.0.1:3001", "ws://127.0.0.1:3001", false);

    assert_eq!(validate_native_endpoint_bases(&endpoint), Ok(()));
    assert_eq!(
        validate_native_endpoint_ready(&endpoint),
        Err(NativeAdapterError::SessionNotBound)
    );
}

#[test]
fn native_shell_mode_local_backend_is_default_and_starts_local_backend() {
    let mode = NativeShellMode::local_backend_default();

    assert_eq!(mode, NativeShellMode::LocalBackend);
    assert!(mode.starts_local_backend());
}

#[test]
fn remote_browser_accepts_https_origin_only() {
    let mode = NativeShellMode::remote_browser("https://deve.example");
    let NativeShellMode::RemoteBrowser { target } = mode else {
        panic!("remote browser mode expected");
    };

    assert_eq!(validate_native_remote_target(&target), Ok(()));
    assert_eq!(
        validate_native_remote_target(&NativeRemoteTarget {
            https_origin: "https://deve.example:443".to_string(),
        }),
        Ok(())
    );
    assert_eq!(
        validate_native_remote_target(&NativeRemoteTarget {
            https_origin: "https://[::1]:8443".to_string(),
        }),
        Ok(())
    );

    for invalid in [
        "http://deve.example",
        "https://user@deve.example",
        "https://deve.example/",
        "https://deve.example/app",
        "https://deve.example?token=secret",
        "https://deve.example#fragment",
        "https://deve.example:0",
        "https://:443",
        "https://bad host",
        "https://deve.example\\app",
        "https://deve.example:443:evil",
        "https://[::1",
        "https://[]",
        " https://deve.example",
    ] {
        let target = NativeRemoteTarget {
            https_origin: invalid.to_string(),
        };
        assert!(
            validate_native_remote_target(&target).is_err(),
            "invalid remote target accepted: {invalid}"
        );
    }
}

// SET-007A: native backend preference is host-local shell config. The preference
// contract below (defaults to local, canonicalizes local by dropping any remote_url,
// remote requires a valid bare https origin) is the automated half of the case; the
// browser-section-unavailable and route/storage-absent checks are the manual Chrome
// walkthrough.
#[test]
fn native_backend_preference_defaults_to_local_backend() {
    let preference = NativeBackendPreference::default();

    assert_eq!(preference, NativeBackendPreference::local());
    assert_eq!(
        native_shell_mode_for_backend_preference(&preference),
        Ok(NativeShellMode::LocalBackend)
    );
    assert_eq!(validate_native_backend_preference(&preference), Ok(()));
}

#[test]
fn native_backend_preference_canonicalizes_local_without_remote_url() {
    let preference = NativeBackendPreference {
        mode: NativeBackendMode::Local,
        remote_url: Some("https://deve.example".into()),
    };

    assert_eq!(preference.canonicalized(), NativeBackendPreference::local());
}

#[test]
fn native_backend_preference_remote_maps_to_remote_browser() {
    let preference = NativeBackendPreference::remote("https://deve.example:8443");

    let shell_mode =
        native_shell_mode_for_backend_preference(&preference).expect("remote preference");
    let NativeShellMode::RemoteBrowser { target } = shell_mode else {
        panic!("remote preference must map to RemoteBrowser");
    };

    assert_eq!(target.https_origin, "https://deve.example:8443");
    assert_eq!(validate_native_backend_preference(&preference), Ok(()));
}

#[test]
fn native_backend_preference_remote_requires_valid_https_origin() {
    for preference in [
        NativeBackendPreference {
            mode: NativeBackendMode::Remote,
            remote_url: None,
        },
        NativeBackendPreference {
            mode: NativeBackendMode::Remote,
            remote_url: Some(String::new()),
        },
        NativeBackendPreference::remote("http://deve.example"),
        NativeBackendPreference::remote("https://deve.example/"),
        NativeBackendPreference::remote("https://deve.example/app"),
    ] {
        assert!(
            validate_native_backend_preference(&preference).is_err(),
            "invalid preference accepted: {preference:?}"
        );
    }
}

#[test]
fn native_backend_validation_result_serializes_without_secret_fields() {
    let ok = NativeBackendValidationResult::success("https://deve.example", "full-peer");
    let json = serde_json::to_string(&ok).expect("validation result json");

    assert!(json.contains("https://deve.example"));
    assert!(json.contains("full-peer"));
    assert!(!json.to_ascii_lowercase().contains("token"));
    assert!(!json.to_ascii_lowercase().contains("secret"));
}

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
