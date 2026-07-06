use super::super::{CWD_LOCK, CwdGuard, EnvGuard};
use crate::config::Config;

#[test]
fn load_checked_fails_closed_on_invalid_mem_cache_env_alias() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_PROFILE", None),
        ("DEVE_MEM_CACHE_MB", None),
        ("MEM_CACHE_MB", Some("not-a-number")),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let err = Config::load_checked().expect_err("invalid mem cache alias");

    assert!(
        err.to_string()
            .contains("Invalid integer environment variable MEM_CACHE_MB")
    );
}

#[test]
fn load_checked_fails_closed_on_invalid_config_file() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(dir.path().join("config.toml"), "snapshot_depth = [").expect("bad config");

    let err = Config::load_checked().expect_err("invalid config must fail closed");

    assert!(
        err.to_string().contains("Failed to build configuration")
            || err.to_string().contains("Failed to parse configuration")
    );
}

#[test]
fn load_checked_fails_closed_on_invalid_runtime_enum_value() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(dir.path().join("config.toml"), "profile = \"debug\"\n").expect("bad profile");

    let err = Config::load_checked().expect_err("invalid profile must fail closed");

    assert!(err.to_string().contains("Failed to parse configuration"));
}

#[test]
fn load_checked_fails_closed_on_invalid_agent_bridge_alias_bool() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_AI_AGENT_BRIDGE_ENABLED", Some("maybe")),
        ("DEVE_AI_AGENT_BRIDGE_TRUSTED", None),
        ("DEVE_AI__AGENT_BRIDGE__ENABLED", None),
        ("DEVE_AI__AGENT_BRIDGE__TRUSTED", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let err = Config::load_checked().expect_err("invalid alias bool");

    assert!(
        err.to_string()
            .contains("Invalid boolean environment variable DEVE_AI_AGENT_BRIDGE_ENABLED")
    );
}

#[test]
fn load_checked_rejects_deve_vault_path_env() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[("DEVE_VAULT_PATH", Some("/tmp/deve-vault"))]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let err = Config::load_checked().expect_err("DEVE_VAULT_PATH must fail closed");

    assert!(
        err.to_string()
            .contains("DEVE_VAULT_PATH is no longer supported")
    );
}

#[test]
fn load_checked_rejects_vault_path_config_key() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[("DEVE_VAULT_PATH", None)]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());
    std::fs::write(dir.path().join("config.toml"), "vault_path = \"vault\"\n").expect("config");

    let err = Config::load_checked().expect_err("vault_path must fail closed");

    assert!(
        err.to_string()
            .contains("vault_path is no longer supported")
    );
}
