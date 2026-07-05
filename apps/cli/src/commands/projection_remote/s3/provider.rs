//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

#[cfg(test)]
use super::credentials::S3Credentials;
use super::credentials::{S3CredentialSource, S3RegionSource};
use super::pull;
use super::push;
use super::transport::{ReqwestS3Transport, S3Transport};
use chrono::{DateTime, Utc};
use deve_core::remote_projection::{
    RemoteProjectionProvider, RemoteProjectionProviderAdapter, RemoteProjectionProviderError,
    RemoteProjectionPullOutcome, RemoteProjectionPullRequest, RemoteProjectionPushOutcome,
    RemoteProjectionPushRequest,
};

pub(crate) struct S3ProjectionProvider<T = ReqwestS3Transport> {
    pub(super) transport: T,
    pub(super) credentials: S3CredentialSource,
    pub(super) region: S3RegionSource,
    pub(super) now: fn() -> DateTime<Utc>,
}

impl S3ProjectionProvider<ReqwestS3Transport> {
    pub(crate) fn new() -> Result<Self, RemoteProjectionProviderError> {
        Ok(Self {
            transport: ReqwestS3Transport::new()?,
            credentials: S3CredentialSource::Env,
            region: S3RegionSource::Env,
            now: Utc::now,
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
            now,
        }
    }
}

impl<T: S3Transport> RemoteProjectionProviderAdapter for S3ProjectionProvider<T> {
    fn provider(&self) -> RemoteProjectionProvider {
        RemoteProjectionProvider::S3
    }

    fn push(
        &mut self,
        request: RemoteProjectionPushRequest,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        push::push_request(
            &self.transport,
            &self.credentials.resolve()?,
            &self.region.resolve()?,
            self.now,
            request,
        )
    }

    fn pull(
        &self,
        request: RemoteProjectionPullRequest,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        pull::pull_request(
            &self.transport,
            &self.credentials.resolve()?,
            &self.region.resolve()?,
            self.now,
            request,
        )
    }
}

pub(crate) struct FailClosedS3ProjectionProvider;
