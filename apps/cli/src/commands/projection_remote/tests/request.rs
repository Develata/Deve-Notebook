//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::super::{
    ProjectionRemoteAction, ProjectionRemoteDirectionAction, S3ProjectionProfileAction,
    S3ProjectionRemoteAction, request_from_action, run_s3_profile_action,
};
use deve_core::remote_projection::{RemoteProjectionDirection, RemoteProjectionProvider};

#[test]
fn webdav_push_builds_provider_request() {
    let request = request_from_action(ProjectionRemoteAction::Webdav {
        action: ProjectionRemoteDirectionAction::Push {
            repo: Some("default".into()),
            locator: "webdav+https://dav.example.com/notebooks/main".into(),
        },
    });

    assert_eq!(request.provider, RemoteProjectionProvider::WebDav);
    assert_eq!(request.direction, RemoteProjectionDirection::Push);
    assert_eq!(request.repo.as_deref(), Some("default"));
}

#[test]
fn s3_pull_builds_provider_request() {
    let request = request_from_action(ProjectionRemoteAction::S3 {
        action: S3ProjectionRemoteAction::Pull {
            repo: None,
            locator: "s3://bucket/notebooks/main".into(),
            profile: None,
        },
    });

    assert_eq!(request.provider, RemoteProjectionProvider::S3);
    assert_eq!(request.direction, RemoteProjectionDirection::Pull);
    assert_eq!(request.locator, "s3://bucket/notebooks/main");
}

#[test]
fn s3_profile_put_writes_host_local_secret_free_profile_store() {
    let dir = tempfile::tempdir().expect("tempdir");

    run_s3_profile_action(
        dir.path(),
        &S3ProjectionProfileAction::Put {
            profile: "minio".into(),
            endpoint_origin: "https://minio.example.com".into(),
            bucket: "bucket".into(),
            allowed_prefix: "notebooks/main".into(),
            region: "us-east-1".into(),
            credential_env_prefix: "MINIO".into(),
            allowed_capabilities: vec!["push".into(), "source-acquisition".into()],
        },
    )
    .expect("profile put");

    let path = dir
        .path()
        .join(".host")
        .join("remote-projection-s3-profiles.toml");
    let content = std::fs::read_to_string(path).expect("profile store");
    let profile = super::super::s3::load_remote_projection_s3_profile(dir.path(), "minio")
        .expect("load profile");
    assert_eq!(profile.profile_id, "minio");
    assert_eq!(profile.credential_ref.env_prefix, "MINIO");
    assert_eq!(
        profile.allowed_capabilities,
        vec!["push", "source-acquisition"]
    );
    assert!(!content.contains("MINIO_SECRET_ACCESS_KEY"));
    assert!(!content.contains("secret_access_key"));
}
