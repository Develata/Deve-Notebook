//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 14_commands#cli-commands
//!
//! CLI S3 adapter for Markdown Projection Workspace push/pull.

mod credentials;
mod list;
mod profile;
mod provider;
mod pull;
mod push;
mod signing;
mod transport;
mod url;

pub(crate) use profile::{
    RemoteProjectionS3Profile, load_remote_projection_s3_profile,
    load_remote_projection_s3_profiles, write_remote_projection_s3_profile,
};
pub(crate) use provider::FailClosedS3ProjectionProvider;
pub(crate) use provider::S3ProjectionProvider;
pub(crate) use pull::S3ProjectionPullAdapter;
pub(crate) use push::S3ProjectionPushAdapter;

pub(crate) trait S3ProjectionAdapter:
    S3ProjectionPushAdapter + S3ProjectionPullAdapter
{
}

impl<T> S3ProjectionAdapter for T where T: S3ProjectionPushAdapter + S3ProjectionPullAdapter {}

#[cfg(test)]
mod tests;
