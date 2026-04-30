use super::AgentBridgePolicy;

#[test]
fn disabled_policy_fails_closed() {
    let policy = AgentBridgePolicy {
        enabled: false,
        trusted: false,
        native_enabled: true,
        requested_mode: "native".to_string(),
        cli_path: Some("agent".to_string()),
        timeout_ms: 30_000,
    };
    assert_eq!(
        policy.spawn_path().expect_err("must fail"),
        "external agent disabled"
    );
}

#[test]
fn untrusted_policy_requires_trusted_mode() {
    let policy = AgentBridgePolicy {
        enabled: true,
        trusted: false,
        native_enabled: true,
        requested_mode: "native".to_string(),
        cli_path: Some("agent".to_string()),
        timeout_ms: 30_000,
    };
    assert_eq!(
        policy.spawn_path().expect_err("must fail"),
        "trusted mode required"
    );
}

#[test]
fn trusted_policy_requires_explicit_cli_path() {
    let policy = AgentBridgePolicy {
        enabled: true,
        trusted: true,
        native_enabled: true,
        requested_mode: "trusted-cli".to_string(),
        cli_path: None,
        timeout_ms: 30_000,
    };
    assert_eq!(
        policy.spawn_path().expect_err("must fail"),
        "AGENT_CLI_PATH required"
    );
}

#[test]
fn trusted_policy_requires_absolute_cli_path() {
    let policy = AgentBridgePolicy {
        enabled: true,
        trusted: true,
        native_enabled: true,
        requested_mode: "trusted-cli".to_string(),
        cli_path: Some("agent".to_string()),
        timeout_ms: 30_000,
    };
    assert_eq!(
        policy.spawn_path().expect_err("must fail"),
        "AGENT_CLI_PATH must be absolute"
    );
}

#[test]
fn trusted_policy_requires_existing_executable_cli_path() {
    let policy = AgentBridgePolicy {
        enabled: true,
        trusted: true,
        native_enabled: true,
        requested_mode: "trusted-cli".to_string(),
        cli_path: Some("/definitely/missing/agent".to_string()),
        timeout_ms: 30_000,
    };
    assert_eq!(
        policy.spawn_path().expect_err("must fail"),
        "AGENT_CLI_PATH must point to an executable file"
    );
}

#[test]
fn trusted_policy_exposes_run_config() {
    let cli_path = std::env::current_exe()
        .expect("current exe")
        .to_string_lossy()
        .into_owned();
    let policy = AgentBridgePolicy {
        enabled: true,
        trusted: true,
        native_enabled: true,
        requested_mode: "trusted-cli".to_string(),
        cli_path: Some(cli_path.clone()),
        timeout_ms: 0,
    };
    let config = policy.run_config().expect("config");
    assert_eq!(config.cli_path, cli_path);
    assert_eq!(config.timeout_ms, 1);
}

#[test]
fn capabilities_fall_back_to_native_when_trusted_cli_is_unavailable() {
    let policy = AgentBridgePolicy {
        enabled: false,
        trusted: false,
        native_enabled: true,
        requested_mode: "trusted-cli".to_string(),
        cli_path: None,
        timeout_ms: 30_000,
    };

    let capabilities = policy.capabilities();

    assert!(capabilities.native_available);
    assert!(!capabilities.trusted_cli_available);
    assert_eq!(capabilities.effective_backend, "native");
    assert_eq!(
        capabilities.effective_backend_reason.as_deref(),
        Some("external agent disabled")
    );
}

#[test]
fn capabilities_report_no_backend_when_native_and_trusted_cli_are_unavailable() {
    let policy = AgentBridgePolicy {
        enabled: false,
        trusted: false,
        native_enabled: false,
        requested_mode: "native".to_string(),
        cli_path: None,
        timeout_ms: 30_000,
    };

    let capabilities = policy.capabilities();

    assert!(!capabilities.native_available);
    assert!(!capabilities.trusted_cli_available);
    assert_eq!(capabilities.effective_backend, "none");
    assert_eq!(
        capabilities.native_reason.as_deref(),
        Some("native AI disabled by config")
    );
}

#[test]
fn capabilities_do_not_promote_native_mode_to_trusted_cli_when_native_is_disabled() {
    let cli_path = std::env::current_exe()
        .expect("current exe")
        .to_string_lossy()
        .into_owned();
    let policy = AgentBridgePolicy {
        enabled: true,
        trusted: true,
        native_enabled: false,
        requested_mode: "native".to_string(),
        cli_path: Some(cli_path),
        timeout_ms: 30_000,
    };

    let capabilities = policy.capabilities();

    assert!(!capabilities.native_available);
    assert!(capabilities.trusted_cli_available);
    assert_eq!(capabilities.effective_backend, "none");
    assert_eq!(
        capabilities.effective_backend_reason.as_deref(),
        Some("native AI disabled by config")
    );
}

#[test]
fn capabilities_keep_requested_trusted_cli_reason_when_policy_blocks_it() {
    let policy = AgentBridgePolicy {
        enabled: true,
        trusted: true,
        native_enabled: true,
        requested_mode: "trusted-cli".to_string(),
        cli_path: Some("/definitely/missing/agent".to_string()),
        timeout_ms: 30_000,
    };

    let capabilities = policy.capabilities();

    assert!(capabilities.native_available);
    assert!(!capabilities.trusted_cli_available);
    assert_eq!(capabilities.effective_backend, "native");
    assert_eq!(
        capabilities.effective_backend_reason.as_deref(),
        Some("AGENT_CLI_PATH must point to an executable file")
    );
}
