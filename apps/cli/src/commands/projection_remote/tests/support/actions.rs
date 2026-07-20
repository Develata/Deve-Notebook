//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::super::super::{
    ProjectionRemoteAction, ProjectionRemoteDirectionAction, S3ProjectionRemoteAction,
};

pub(in crate::commands::projection_remote::tests) fn webdav_push_action() -> ProjectionRemoteAction
{
    // repo: None resolves to the single cataloged local repo (machine names are
    // canonical RepoId strings; "default" is only a host-local display alias).
    ProjectionRemoteAction::Webdav {
        action: ProjectionRemoteDirectionAction::Push {
            repo: None,
            locator: "webdav+https://dav.example.com/notebooks/main".into(),
        },
    }
}

pub(in crate::commands::projection_remote::tests) fn s3_push_action() -> ProjectionRemoteAction {
    ProjectionRemoteAction::S3 {
        action: S3ProjectionRemoteAction::Push {
            repo: None,
            locator: "s3://bucket/notebooks/main".into(),
            profile: None,
        },
    }
}
