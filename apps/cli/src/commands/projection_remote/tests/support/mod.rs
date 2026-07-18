//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

mod actions;
mod harness;
mod s3_providers;
mod webdav_push_providers;

pub(super) use actions::{s3_push_action, webdav_push_action};
pub(super) use harness::initialized_default_repo;
pub(super) use s3_providers::RecordingS3Provider;
pub(super) use webdav_push_providers::{
    AuthoritativeMetadataPushProvider, AuthorityEffectPushProvider, FailingProvider,
    RecordingProvider,
};
