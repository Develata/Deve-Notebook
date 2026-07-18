//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract
//!
//! Shared S3 adapter for Projection push and ordered source acquisition.

mod credentials;
mod list;
mod profile;
mod provider;
mod push;
mod signing;
mod source;
mod transport;
mod url;

pub(crate) use profile::{
    RemoteProjectionS3Profile, load_remote_projection_s3_profile,
    load_remote_projection_s3_profiles, write_remote_projection_s3_profile,
};
pub(crate) use provider::FailClosedS3ProjectionProvider;
pub(crate) use provider::S3ProjectionProvider;
pub(crate) use push::S3ProjectionPushAdapter;

pub(crate) fn reject_unprofiled_custom_endpoint(
    locator: &str,
) -> Result<(), deve_core::remote_projection::RemoteProjectionProviderError> {
    url::reject_custom_https_endpoint_without_binding(locator)
}

#[cfg(test)]
mod tests;
