use super::Config;
use std::sync::Mutex;

static CWD_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_default_config() {
    let config = Config::load();
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
