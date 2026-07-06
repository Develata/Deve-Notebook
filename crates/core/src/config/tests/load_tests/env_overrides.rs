use super::super::{CWD_LOCK, CwdGuard, EnvGuard};
use crate::config::{Config, SyncMode};

#[test]
fn env_overrides_flat_underscore_keys() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_LEDGER_DIR", Some("/tmp/deve-ledger")),
        ("DEVE_SYNC_MODE", Some("manual")),
        ("MEM_CACHE_MB", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let config = Config::load_checked().expect("env config");

    assert_eq!(config.ledger_dir, "/tmp/deve-ledger");
    assert_eq!(config.sync_mode, SyncMode::Manual);
}
