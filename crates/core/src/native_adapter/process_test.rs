use super::*;

fn valid_spawn_spec() -> NativeProcessSpawnSpec {
    let root = std::env::current_dir().expect("current dir");
    NativeProcessSpawnSpec {
        executable: root.join("target/native/deve_cli"),
        argv: vec!["serve".to_string(), "--dev".to_string()],
        cwd: root.clone(),
        env_allowlist: vec!["DEVE_PROFILE".to_string(), "MEM_CACHE_MB".to_string()],
        env: vec![NativeProcessEnvBinding {
            key: "DEVE_PROFILE".to_string(),
            value: "standard".to_string(),
        }],
        profile: "standard".to_string(),
        config_path: root.join("config.toml"),
        vault_path: root.join("vault"),
        ledger_path: root.join("ledger"),
        bind_hints: NativeProcessBindHints {
            http_host: "127.0.0.1".to_string(),
            http_port: Some(3001),
            ws_host: "localhost".to_string(),
            ws_port: Some(3001),
        },
        path_resolution: NativeProcessPathResolution::AbsoluteOnly,
    }
}

fn endpoint(http_base: &str, ws_base: &str) -> NativeEndpointReady {
    NativeEndpointReady {
        http_base: http_base.to_string(),
        ws_base: ws_base.to_string(),
        node_role: "native-main".to_string(),
        session_bound: true,
    }
}

#[test]
fn current_policy_defers_real_process_runtime() {
    let policy = CURRENT_NATIVE_PROCESS_ADAPTER_POLICY;

    assert!(policy.is_deferred_no_runtime());
    assert!(!policy.child_process_runtime_enabled);
    assert!(policy.packaging_gate_required);
    assert!(!policy.authority_writes_allowed);
}

#[test]
fn default_adapter_rejects_child_process_runtime() {
    let adapter = NativeProcessAdapter::default();
    let snapshot = adapter.snapshot();

    assert!(adapter.ensure_child_process_runtime_enabled().is_err());
    assert!(snapshot.is_default_safe_boundary());
    assert_eq!(snapshot.state, NativeProcessAdapterState::Deferred);
}

#[test]
fn default_adapter_binds_existing_loopback_service_without_runtime() {
    let mut adapter = NativeProcessAdapter::default();

    let snapshot = adapter
        .bind_existing_endpoint(endpoint("http://127.0.0.1:3001", "ws://localhost:3001"))
        .expect("bind endpoint");

    assert_eq!(
        snapshot.state,
        NativeProcessAdapterState::ExistingEndpointBound
    );
    assert!(snapshot.health_probe.is_healthy());
    assert!(!snapshot.endpoint.as_ref().expect("endpoint").session_bound);
    assert!(snapshot.is_default_safe_boundary());
}

#[test]
fn session_handoff_requires_existing_endpoint() {
    let mut adapter = NativeProcessAdapter::default();

    assert_eq!(
        adapter.bind_session(true),
        Err(NativeProcessAdapterError::EndpointNotBound)
    );

    adapter
        .bind_existing_endpoint(endpoint("http://127.0.0.1:3001", "ws://127.0.0.1:3001"))
        .expect("bind endpoint");

    assert_eq!(
        adapter.bind_session(false),
        Err(NativeProcessAdapterError::SessionNotBound)
    );

    let snapshot = adapter.bind_session(true).expect("bind session");
    assert_eq!(
        snapshot.state,
        NativeProcessAdapterState::SessionHandoffReady
    );
    assert!(snapshot.endpoint.expect("endpoint").session_bound);
}

#[test]
fn default_adapter_rejects_non_loopback_existing_endpoint() {
    let mut adapter = NativeProcessAdapter::default();

    assert!(matches!(
        adapter.bind_existing_endpoint(endpoint("http://192.168.1.10:3001", "ws://127.0.0.1:3001")),
        Err(NativeProcessAdapterError::InvalidEndpoint(
            NativeAdapterError::NonLoopbackHost { field: "http_base" }
        ))
    ));
}

#[test]
fn process_spawn_spec_rejects_empty_executable() {
    let mut spec = valid_spawn_spec();
    spec.executable = std::path::PathBuf::new();

    assert_eq!(
        spec.validate_contract(),
        Err(NativeProcessRuntimeError::EmptyPath {
            field: "executable"
        })
    );
}

#[test]
fn process_spawn_spec_rejects_relative_executable_without_resolver() {
    let mut spec = valid_spawn_spec();
    spec.executable = "deve_cli".into();

    assert_eq!(
        spec.validate_contract(),
        Err(NativeProcessRuntimeError::RelativePathForbidden {
            field: "executable"
        })
    );
}

#[test]
fn process_spawn_spec_rejects_unknown_environment_variable() {
    let mut spec = valid_spawn_spec();
    spec.env.push(NativeProcessEnvBinding {
        key: "AUTH_SECRET".to_string(),
        value: "must-not-be-forwarded".to_string(),
    });

    assert_eq!(
        spec.validate_contract(),
        Err(
            NativeProcessRuntimeError::EnvironmentVariableNotAllowlisted {
                key: "AUTH_SECRET".to_string()
            }
        )
    );
}

#[test]
fn process_spawn_spec_rejects_non_loopback_bind_hints() {
    let mut spec = valid_spawn_spec();
    spec.bind_hints.http_host = "0.0.0.0".to_string();

    assert_eq!(
        spec.validate_contract(),
        Err(NativeProcessRuntimeError::NonLoopbackBindHost { field: "http_host" })
    );
}

#[test]
fn process_runtime_snapshot_never_serializes_secret_token_or_output_payload() {
    let snapshot =
        NativeProcessRuntimeSnapshot::disabled_by_policy(CURRENT_NATIVE_PROCESS_ADAPTER_POLICY);
    let encoded = serde_json::to_string(&snapshot).expect("serialize snapshot");

    assert_eq!(snapshot.state, NativeProcessRuntimeState::Disabled);
    assert!(!snapshot.child_process_runtime_enabled);
    assert!(!snapshot.authority_writes_allowed);
    assert!(!encoded.contains("secret"));
    assert!(!encoded.contains("token"));
    assert!(!encoded.contains("stdout"));
    assert!(!encoded.contains("stderr"));
}

#[test]
fn process_env_binding_debug_and_serde_redact_secret_values() {
    let binding = NativeProcessEnvBinding {
        key: NATIVE_SESSION_BOOTSTRAP_SECRET_ENV.to_string(),
        value: "native-secret-value".to_string(),
    };

    let debug = format!("{binding:?}");
    let encoded = serde_json::to_string(&binding).expect("serialize env binding");

    assert!(debug.contains("<redacted>"));
    assert!(encoded.contains("<redacted>"));
    assert!(!debug.contains("native-secret-value"));
    assert!(!encoded.contains("native-secret-value"));
}

#[test]
fn process_runtime_failure_contract_marks_only_budgeted_failures_retryable() {
    assert!(NativeProcessRuntimeFailureKind::BindFailed.retryable_by_default());
    assert!(NativeProcessRuntimeFailureKind::HealthProbeFailed.retryable_by_default());
    assert!(NativeProcessRuntimeFailureKind::ProcessExited.retryable_by_default());
    assert!(!NativeProcessRuntimeFailureKind::SessionHandoffFailed.retryable_by_default());
    assert!(!NativeProcessRuntimeFailureKind::SpawnExecutableMissing.retryable_by_default());
    assert!(!NativeProcessRuntimeFailureKind::EnvironmentPolicyViolation.retryable_by_default());
}
