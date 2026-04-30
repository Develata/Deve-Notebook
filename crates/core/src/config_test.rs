use super::{Config, SyncMode};
use std::ffi::OsString;
use std::sync::Mutex;

static CWD_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_default_config() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_AI__MODE", None),
        ("DEVE_AI__NATIVE_ENABLED", None),
        ("DEVE_AI__AGENT_BRIDGE__ENABLED", None),
        ("DEVE_AI__AGENT_BRIDGE__TRUSTED", None),
        ("AGENT_CLI_PATH", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let old_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("set cwd");

    let config = Config::load();

    std::env::set_current_dir(old_cwd).expect("restore cwd");
    assert!(!config.ledger_dir.is_empty());
    assert_eq!(config.ai.mode, "native");
    assert!(config.ai.native_enabled);
    assert!(!config.ai.agent_bridge.enabled);
    assert!(!config.ai.agent_bridge.trusted);
}

#[test]
fn load_checked_fails_closed_on_invalid_config_file() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let dir = tempfile::tempdir().expect("tempdir");
    let old_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("set cwd");
    std::fs::write(dir.path().join("config.toml"), "snapshot_depth = [").expect("bad config");

    let err = Config::load_checked().expect_err("invalid config must fail closed");

    std::env::set_current_dir(old_cwd).expect("restore cwd");
    assert!(
        err.to_string().contains("Failed to build configuration")
            || err.to_string().contains("Failed to parse configuration")
    );
}

#[test]
fn env_overrides_flat_underscore_keys() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set(&[
        ("DEVE_LEDGER_DIR", "/tmp/deve-ledger"),
        ("DEVE_VAULT_PATH", "/tmp/deve-vault"),
        ("DEVE_SYNC_MODE", "manual"),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let old_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("set cwd");

    let config = Config::load_checked().expect("env config");

    std::env::set_current_dir(old_cwd).expect("restore cwd");
    assert_eq!(config.ledger_dir, "/tmp/deve-ledger");
    assert_eq!(config.vault_path, "/tmp/deve-vault");
    assert_eq!(config.sync_mode, SyncMode::Manual);
}

#[test]
fn trusted_cli_requested_mode_is_preserved_when_policy_conditions_are_missing() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("AGENT_CLI_PATH", None),
        ("DEVE_AI__AGENT_BRIDGE__ENABLED", None),
        ("DEVE_AI__AGENT_BRIDGE__TRUSTED", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let old_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("set cwd");
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

    std::env::set_current_dir(old_cwd).expect("restore cwd");
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
    let old_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("set cwd");
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

    std::env::set_current_dir(old_cwd).expect("restore cwd");
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
    let old_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("set cwd");
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

    std::env::set_current_dir(old_cwd).expect("restore cwd");
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
    let old_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("set cwd");
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

    std::env::set_current_dir(old_cwd).expect("restore cwd");
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
    let old_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("set cwd");
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

    std::env::set_current_dir(old_cwd).expect("restore cwd");
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
    let old_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("set cwd");
    std::fs::write(
        dir.path().join("config.toml"),
        r#"
[ai]
mode = "trusted-cli"
"#,
    )
    .expect("write config");

    let config = Config::load_checked().expect("load config");

    std::env::set_current_dir(old_cwd).expect("restore cwd");
    assert_eq!(config.ai.mode, "trusted-cli");
    assert!(config.ai.agent_bridge.enabled);
    assert!(config.ai.agent_bridge.trusted);
}

struct EnvGuard {
    previous: Vec<(&'static str, Option<OsString>)>,
}

impl EnvGuard {
    fn set(entries: &[(&'static str, &'static str)]) -> Self {
        let previous = entries
            .iter()
            .map(|(key, value)| {
                let old = std::env::var_os(key);
                unsafe {
                    std::env::set_var(key, value);
                }
                (*key, old)
            })
            .collect();
        Self { previous }
    }

    fn set_optional(entries: &[(&'static str, Option<&str>)]) -> Self {
        let previous = entries
            .iter()
            .map(|(key, value)| {
                let old = std::env::var_os(key);
                unsafe {
                    match value {
                        Some(value) => std::env::set_var(key, value),
                        None => std::env::remove_var(key),
                    }
                }
                (*key, old)
            })
            .collect();
        Self { previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.previous.drain(..) {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}
