use super::{CWD_LOCK, CwdGuard, EnvGuard};
use crate::config::Config;

#[test]
fn trusted_cli_requested_mode_is_preserved_when_policy_conditions_are_missing() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("AGENT_CLI_PATH", None),
        ("DEVE_AI__AGENT_BRIDGE__ENABLED", None),
        ("DEVE_AI__AGENT_BRIDGE__TRUSTED", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(
        dir.path().join("config.toml"),
        r#"
[ai]
mode = "trusted-cli"

[ai.agent_bridge]
enabled = true
trusted = false
"#,
    )
    .expect("write config");

    let config = Config::load_checked().expect("load config");

    assert_eq!(config.ai.mode, "trusted-cli");
}

#[test]
fn trusted_cli_requested_mode_is_preserved_when_agent_cli_path_is_missing() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("AGENT_CLI_PATH", None),
        ("DEVE_AI__AGENT_BRIDGE__ENABLED", None),
        ("DEVE_AI__AGENT_BRIDGE__TRUSTED", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(
        dir.path().join("config.toml"),
        r#"
[ai]
mode = "trusted-cli"

[ai.agent_bridge]
enabled = true
trusted = true
"#,
    )
    .expect("write config");

    let config = Config::load_checked().expect("load config");

    assert_eq!(config.ai.mode, "trusted-cli");
}

#[test]
fn trusted_cli_requested_mode_is_preserved_when_agent_cli_path_is_relative() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("AGENT_CLI_PATH", Some("agent")),
        ("DEVE_AI__AGENT_BRIDGE__ENABLED", None),
        ("DEVE_AI__AGENT_BRIDGE__TRUSTED", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(
        dir.path().join("config.toml"),
        r#"
[ai]
mode = "trusted-cli"

[ai.agent_bridge]
enabled = true
trusted = true
"#,
    )
    .expect("write config");

    let config = Config::load_checked().expect("load config");

    assert_eq!(config.ai.mode, "trusted-cli");
}

#[test]
fn trusted_cli_requested_mode_is_preserved_when_agent_cli_path_is_not_executable() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let dir = tempfile::tempdir().expect("tempdir");
    let cli_path = dir.path().join("agent.txt");
    std::fs::write(&cli_path, "not executable").expect("write fake agent");
    let cli_path = cli_path.to_string_lossy().into_owned();
    let _env = EnvGuard::set_optional(&[
        ("AGENT_CLI_PATH", Some(cli_path.as_str())),
        ("DEVE_AI__AGENT_BRIDGE__ENABLED", None),
        ("DEVE_AI__AGENT_BRIDGE__TRUSTED", None),
    ]);
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(
        dir.path().join("config.toml"),
        r#"
[ai]
mode = "trusted-cli"

[ai.agent_bridge]
enabled = true
trusted = true
"#,
    )
    .expect("write config");

    let config = Config::load_checked().expect("load config");

    assert_eq!(config.ai.mode, "trusted-cli");
}

#[test]
fn trusted_cli_mode_is_kept_when_policy_conditions_are_satisfied() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let cli_path = std::env::current_exe()
        .expect("current exe")
        .to_string_lossy()
        .into_owned();
    let _env = EnvGuard::set_optional(&[
        ("AGENT_CLI_PATH", Some(cli_path.as_str())),
        ("DEVE_AI__AGENT_BRIDGE__ENABLED", None),
        ("DEVE_AI__AGENT_BRIDGE__TRUSTED", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(
        dir.path().join("config.toml"),
        r#"
[ai]
mode = "trusted-cli"

[ai.agent_bridge]
enabled = true
trusted = true
"#,
    )
    .expect("write config");

    let config = Config::load_checked().expect("load config");

    assert_eq!(config.ai.mode, "trusted-cli");
}

#[test]
fn trusted_cli_mode_honors_agent_bridge_env_aliases() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let cli_path = std::env::current_exe()
        .expect("current exe")
        .to_string_lossy()
        .into_owned();
    let _env = EnvGuard::set_optional(&[
        ("AGENT_CLI_PATH", Some(cli_path.as_str())),
        ("DEVE_AI_AGENT_BRIDGE_ENABLED", Some("true")),
        ("DEVE_AI_AGENT_BRIDGE_TRUSTED", Some("true")),
        ("DEVE_AI__AGENT_BRIDGE__ENABLED", None),
        ("DEVE_AI__AGENT_BRIDGE__TRUSTED", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(
        dir.path().join("config.toml"),
        r#"
[ai]
mode = "trusted-cli"
"#,
    )
    .expect("write config");

    let config = Config::load_checked().expect("load config");

    assert_eq!(config.ai.mode, "trusted-cli");
    assert!(config.ai.agent_bridge.enabled);
    assert!(config.ai.agent_bridge.trusted);
}
