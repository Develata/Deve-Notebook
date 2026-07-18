//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 06_backup#projection-backup-secret-ref-contract

use super::*;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn profile_store_roundtrips_secret_free_binding() {
    let dir = tempfile::tempdir().expect("tempdir");
    let profile = RemoteProjectionS3Profile::env_profile(
        "minio",
        "https://minio.example.com",
        "bucket",
        "notebooks/main",
        "us-east-1",
        "MINIO",
        vec!["push".into(), "source-acquisition".into()],
    );

    let path = write_remote_projection_s3_profile(dir.path(), profile).expect("write profile");
    let raw = std::fs::read_to_string(&path).expect("read profile");
    assert!(raw.contains("credential_ref"));
    assert!(raw.contains("MINIO"));
    assert!(!raw.contains("SECRET_ACCESS_KEY"));
    assert!(!raw.contains("AKIDEXAMPLE"));

    let loaded = load_remote_projection_s3_profile(dir.path(), "minio").expect("load profile");
    assert_eq!(loaded.profile_id, "minio");
    assert_eq!(loaded.allowed_prefix, "notebooks/main");
}

#[test]
fn profile_store_rejects_legacy_direction_field_and_missing_capabilities() {
    let profile = RemoteProjectionS3Profile::env_profile(
        "minio",
        "https://minio.example.com",
        "bucket",
        "notebooks/main",
        "us-east-1",
        "MINIO",
        vec!["push".into(), "source-acquisition".into()],
    );
    let current = toml::to_string(&RemoteProjectionS3ProfileStore {
        profiles: vec![profile],
    })
    .expect("serialize profile store");

    for raw in [
        current.replace("allowed_capabilities", "allowed_directions"),
        current
            .lines()
            .filter(|line| !line.starts_with("allowed_capabilities"))
            .collect::<Vec<_>>()
            .join("\n"),
    ] {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = profile_store_path(dir.path());
        std::fs::create_dir_all(path.parent().expect("profile parent")).expect("create parent");
        std::fs::write(&path, raw).expect("write invalid profile");

        let error = load_remote_projection_s3_profiles(dir.path())
            .expect_err("legacy or missing capability field must fail closed");
        assert!(
            error.to_string().contains("failed to parse"),
            "unexpected error: {error}"
        );
    }
}

#[test]
fn profile_binding_matches_origin_bucket_prefix_and_capability() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let _env = EnvGuard::set(&[
        ("DEVE_PROFILE_BINDING_TEST_ACCESS_KEY_ID", Some("minio-key")),
        (
            "DEVE_PROFILE_BINDING_TEST_SECRET_ACCESS_KEY",
            Some("minio-secret"),
        ),
        (
            "DEVE_PROFILE_BINDING_TEST_SESSION_TOKEN",
            Some("minio-token"),
        ),
    ]);
    let profile = RemoteProjectionS3Profile::env_profile(
        "minio",
        "https://minio.example.com/",
        "bucket",
        "notebooks/main",
        "auto",
        "DEVE_PROFILE_BINDING_TEST",
        vec!["push".into()],
    );

    let binding = profile
        .runtime_binding_for(
            TransportCapability::Push,
            "s3+https://minio.example.com/bucket/notebooks/main/sub",
        )
        .expect("binding");

    assert_eq!(binding.region, "auto");
    assert_eq!(binding.credentials.access_key_id, "minio-key");
    assert_eq!(binding.credentials.secret_access_key, "minio-secret");
    assert_eq!(
        binding.credentials.session_token.as_deref(),
        Some("minio-token")
    );

    let err = profile
        .runtime_binding_for(
            TransportCapability::SourceAcquisition,
            "s3+https://minio.example.com/bucket/notebooks/main/sub",
        )
        .expect_err("source acquisition disallowed");
    assert!(
        err.to_string()
            .contains("does not allow source-acquisition")
    );
}

#[test]
fn profile_binding_rejects_prefix_escape_before_credentials() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let _env = EnvGuard::set(&[
        ("DEVE_PROFILE_PREFIX_ESCAPE_TEST_ACCESS_KEY_ID", None),
        ("DEVE_PROFILE_PREFIX_ESCAPE_TEST_SECRET_ACCESS_KEY", None),
        ("DEVE_PROFILE_PREFIX_ESCAPE_TEST_SESSION_TOKEN", None),
    ]);
    let profile = RemoteProjectionS3Profile::env_profile(
        "minio",
        "https://minio.example.com",
        "bucket",
        "notebooks/main",
        "us-east-1",
        "DEVE_PROFILE_PREFIX_ESCAPE_TEST",
        vec!["push".into(), "source-acquisition".into()],
    );

    let err = profile
        .runtime_binding_for(
            TransportCapability::Push,
            "s3+https://minio.example.com/bucket/notebooks/main-escape",
        )
        .expect_err("prefix escape");

    assert!(err.to_string().contains("does not allow locator prefix"));
    assert!(
        !err.to_string()
            .contains("DEVE_PROFILE_PREFIX_ESCAPE_TEST_ACCESS_KEY_ID")
    );
}

struct EnvGuard {
    old: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn set(values: &[(&'static str, Option<&'static str>)]) -> Self {
        let old = values
            .iter()
            .map(|(key, _)| (*key, std::env::var(key).ok()))
            .collect::<Vec<_>>();
        for (key, value) in values {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
        Self { old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.old.drain(..) {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}
