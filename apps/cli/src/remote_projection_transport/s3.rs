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
pub(crate) use provider::S3ProjectionProvider;
pub(crate) use push::S3ProjectionPushAdapter;

pub(crate) fn source_provider_for_locator(
    ledger_dir: &std::path::Path,
    locator: &str,
) -> Result<
    (S3ProjectionProvider, Option<String>),
    deve_core::remote_projection::RemoteProjectionProviderError,
> {
    provider_for_locator(
        ledger_dir,
        super::TransportCapability::SourceAcquisition,
        locator,
    )
}

pub(crate) fn provider_for_locator(
    ledger_dir: &std::path::Path,
    capability: super::TransportCapability,
    locator: &str,
) -> Result<
    (S3ProjectionProvider, Option<String>),
    deve_core::remote_projection::RemoteProjectionProviderError,
> {
    if !url::is_s3_custom_https_locator(locator) {
        return Ok((S3ProjectionProvider::new()?, None));
    }
    let mut matches = Vec::new();
    for profile in load_remote_projection_s3_profiles(ledger_dir)? {
        if profile.ensure_locator_binding(capability, locator).is_ok() {
            matches.push(profile);
        }
    }
    if matches.len() != 1 {
        return Err(
            deve_core::remote_projection::RemoteProjectionProviderError::ProviderIo(format!(
                "S3 custom endpoint {} requires exactly one matching profile; observed {}",
                capability.profile_name(),
                matches.len()
            )),
        );
    }
    let profile = matches.pop().expect("exactly one profile was checked");
    let profile_id = profile.profile_id.clone();
    Ok((
        S3ProjectionProvider::new()?.with_custom_profile(profile),
        Some(profile_id),
    ))
}

#[cfg(test)]
mod tests;
