use super::super::{CWD_LOCK, CwdGuard, EnvGuard};
use crate::config::{AppProfile, Config};

#[test]
fn low_spec_profile_applies_unset_runtime_presets() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_PROFILE", Some("low-spec")),
        ("DEVE_SNAPSHOT_DEPTH", None),
        ("DEVE_MEM_CACHE_MB", None),
        ("MEM_CACHE_MB", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let config = Config::load_checked().expect("low-spec config");

    assert_eq!(config.profile, AppProfile::LowSpec);
    assert_eq!(config.snapshot_depth, 10);
    assert_eq!(config.mem_cache_mb, 32);
}

#[test]
fn explicit_runtime_values_override_profile_presets() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_PROFILE", Some("low-spec")),
        ("DEVE_SNAPSHOT_DEPTH", Some("77")),
        ("DEVE_MEM_CACHE_MB", Some("88")),
        ("MEM_CACHE_MB", None),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let config = Config::load_checked().expect("explicit runtime config");

    assert_eq!(config.profile, AppProfile::LowSpec);
    assert_eq!(config.snapshot_depth, 77);
    assert_eq!(config.mem_cache_mb, 88);
}

#[test]
fn mem_cache_mb_compat_env_alias_overrides_prefixed_config() {
    let _guard = CWD_LOCK.lock().expect("lock cwd");
    let _env = EnvGuard::set_optional(&[
        ("DEVE_PROFILE", Some("low-spec")),
        ("DEVE_MEM_CACHE_MB", Some("88")),
        ("MEM_CACHE_MB", Some("64")),
    ]);
    let dir = tempfile::tempdir().expect("tempdir");
    let _cwd = CwdGuard::enter(dir.path());

    let config = Config::load_checked().expect("mem cache alias config");

    assert_eq!(config.profile, AppProfile::LowSpec);
    assert_eq!(config.mem_cache_mb, 64);
}
