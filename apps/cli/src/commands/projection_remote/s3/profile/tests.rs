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
        vec!["push".into(), "pull".into()],
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
fn profile_binding_matches_origin_bucket_prefix_and_direction() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let _env = EnvGuard::set(&[
        ("MINIO_ACCESS_KEY_ID", Some("minio-key")),
        ("MINIO_SECRET_ACCESS_KEY", Some("minio-secret")),
        ("MINIO_SESSION_TOKEN", Some("minio-token")),
    ]);
    let profile = RemoteProjectionS3Profile::env_profile(
        "minio",
        "https://minio.example.com/",
        "bucket",
        "notebooks/main",
        "auto",
        "MINIO",
        vec!["push".into()],
    );

    let binding = profile
        .runtime_binding_for(
            RemoteProjectionDirection::Push,
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
            RemoteProjectionDirection::Pull,
            "s3+https://minio.example.com/bucket/notebooks/main/sub",
        )
        .expect_err("pull disallowed");
    assert!(err.to_string().contains("does not allow pull"));
}

#[test]
fn profile_binding_rejects_prefix_escape_before_credentials() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let _env = EnvGuard::set(&[
        ("MINIO_ACCESS_KEY_ID", None),
        ("MINIO_SECRET_ACCESS_KEY", None),
        ("MINIO_SESSION_TOKEN", None),
    ]);
    let profile = RemoteProjectionS3Profile::env_profile(
        "minio",
        "https://minio.example.com",
        "bucket",
        "notebooks/main",
        "us-east-1",
        "MINIO",
        vec!["push".into(), "pull".into()],
    );

    let err = profile
        .runtime_binding_for(
            RemoteProjectionDirection::Push,
            "s3+https://minio.example.com/bucket/notebooks/main-escape",
        )
        .expect_err("prefix escape");

    assert!(err.to_string().contains("does not allow locator prefix"));
    assert!(!err.to_string().contains("MINIO_ACCESS_KEY_ID"));
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
