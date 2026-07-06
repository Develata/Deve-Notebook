//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::super::{ProjectionRemoteAction, ProjectionRemoteDirectionAction, request_from_action};
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
        action: ProjectionRemoteDirectionAction::Pull {
            repo: None,
            locator: "s3://bucket/notebooks/main".into(),
        },
    });

    assert_eq!(request.provider, RemoteProjectionProvider::S3);
    assert_eq!(request.direction, RemoteProjectionDirection::Pull);
    assert_eq!(request.locator, "s3://bucket/notebooks/main");
}
