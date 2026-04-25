use super::AgentBridgePolicy;

#[test]
fn disabled_policy_fails_closed() {
    let policy = AgentBridgePolicy {
        enabled: false,
        trusted: false,
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
        cli_path: Some("agent".to_string()),
        timeout_ms: 30_000,
    };
    assert_eq!(
        policy.spawn_path().expect_err("must fail"),
        "AGENT_CLI_PATH must be absolute"
    );
}

#[test]
fn trusted_policy_exposes_run_config() {
    let policy = AgentBridgePolicy {
        enabled: true,
        trusted: true,
        cli_path: Some("/usr/bin/agent".to_string()),
        timeout_ms: 0,
    };
    let config = policy.run_config().expect("config");
    assert_eq!(config.cli_path, "/usr/bin/agent");
    assert_eq!(config.timeout_ms, 1);
}
