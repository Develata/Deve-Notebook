use super::{Config, SyncMode};
use std::ffi::OsString;
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
