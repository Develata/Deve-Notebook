//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract

use super::credentials::{S3CredentialSource, S3Credentials, S3RegionSource};
use super::profile::{RemoteProjectionS3Profile, S3ProfileRuntimeBinding};
use super::transport::ReqwestS3Transport;
use super::url::{
    S3CustomEndpointUrlBinding, custom_endpoint_requires_binding_error, is_s3_custom_https_locator,
};
use crate::remote_projection_transport::TransportCapability;
use chrono::{DateTime, Utc};
use deve_core::remote_projection::RemoteProjectionProviderError;

pub(crate) struct S3ProjectionProvider<T = ReqwestS3Transport> {
    pub(super) transport: T,
    pub(super) credentials: S3CredentialSource,
    pub(super) region: S3RegionSource,
    pub(super) custom_profile: Option<RemoteProjectionS3Profile>,
    pub(super) now: fn() -> DateTime<Utc>,
}

impl S3ProjectionProvider<ReqwestS3Transport> {
    pub(crate) fn new() -> Result<Self, RemoteProjectionProviderError> {
        Ok(Self {
            transport: ReqwestS3Transport::new()?,
            credentials: S3CredentialSource::Env,
            region: S3RegionSource::Env,
            custom_profile: None,
            now: Utc::now,
        })
    }
    pub(crate) fn with_custom_profile(mut self, profile: RemoteProjectionS3Profile) -> Self {
        self.custom_profile = Some(profile);
        self
    }
}

pub(super) struct S3RequestBinding {
    pub(super) credentials: S3Credentials,
    pub(super) region: String,
    pub(super) custom_url_binding: Option<S3CustomEndpointUrlBinding>,
}

impl<T> S3ProjectionProvider<T> {
    pub(super) fn request_binding(
        &self,
        capability: TransportCapability,
        locator: &str,
    ) -> Result<S3RequestBinding, RemoteProjectionProviderError> {
        if is_s3_custom_https_locator(locator) {
            let profile = self
                .custom_profile
                .as_ref()
                .ok_or_else(custom_endpoint_requires_binding_error)?;
            let S3ProfileRuntimeBinding {
                credentials,
                region,
                url_binding,
            } = profile.runtime_binding_for(capability, locator)?;
            return Ok(S3RequestBinding {
                credentials,
                region,
                custom_url_binding: Some(url_binding),
            });
        }
        if self.custom_profile.is_some() {
            return Err(RemoteProjectionProviderError::ProviderIo(
                "Remote Projection S3 profile can only be used with s3+https:// custom endpoint locators".into(),
            ));
        }
        Ok(S3RequestBinding {
            credentials: self.credentials.resolve()?,
            region: self.region.resolve()?,
            custom_url_binding: None,
        })
    }
}

#[cfg(test)]
impl<T> S3ProjectionProvider<T> {
    pub(super) fn new_for_test(
        transport: T,
        credentials: S3Credentials,
        region: impl Into<String>,
        now: fn() -> DateTime<Utc>,
    ) -> Self {
        Self {
            transport,
            credentials: S3CredentialSource::Static(credentials),
            region: S3RegionSource::Static(region.into()),
            custom_profile: None,
            now,
        }
    }

    pub(super) fn new_for_test_with_profile(
        transport: T,
        profile: RemoteProjectionS3Profile,
        now: fn() -> DateTime<Utc>,
    ) -> Self {
        Self {
            transport,
            credentials: S3CredentialSource::Env,
            region: S3RegionSource::Env,
            custom_profile: Some(profile),
            now,
        }
    }
}
