//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::credentials::S3Credentials;
use super::provider::FailClosedS3ProjectionProvider;
use super::provider::S3ProjectionProvider;
use super::signing::signed_put_request;
use super::transport::S3Transport;
use super::url::{reject_custom_https_endpoint_without_binding, s3_file_url};
use crate::commands::projection_remote::collect::MarkdownProjectionFileRef;
use chrono::{DateTime, Utc};
use deve_core::remote_projection::{
    RemoteProjectionAuthorityEffects, RemoteProjectionProvider, RemoteProjectionProviderError,
    RemoteProjectionPushOutcome, RemoteProjectionPushRequest,
};
use std::fs;

pub(crate) trait S3ProjectionPushAdapter {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        locator: &str,
        files: &[MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError>;
}

impl<T: S3Transport> S3ProjectionPushAdapter for S3ProjectionProvider<T> {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        locator: &str,
        files: &[MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        if provider != RemoteProjectionProvider::S3 {
            return Err(RemoteProjectionProviderError::ProviderMismatch);
        }
        let request = RemoteProjectionPushRequest::new(provider, locator, Vec::new())?;
        reject_custom_https_endpoint_without_binding(request.locator())?;
        let credentials = self.credentials.resolve()?;
        let region = self.region.resolve()?;
        for file in files {
            let content = fs::read(file.fs_path()).map_err(|err| {
                RemoteProjectionProviderError::ProviderIo(format!(
                    "failed to read projection file {}: {err}",
                    file.fs_path().display()
                ))
            })?;
            push_payload(
                &self.transport,
                &credentials,
                &region,
                self.now,
                request.locator(),
                file.path(),
                content,
            )?;
        }
        Ok(push_outcome(files.len()))
    }
}

impl S3ProjectionPushAdapter for FailClosedS3ProjectionProvider {
    fn push_projection_files(
        &mut self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
        _files: &[MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        Err(RemoteProjectionProviderError::ProviderIo(
            "S3 push provider is unavailable in this execution path (provider_io_ready=false)"
                .into(),
        ))
    }
}

pub(super) fn push_request<T: S3Transport>(
    transport: &T,
    credentials: &S3Credentials,
    region: &str,
    now: fn() -> DateTime<Utc>,
    request: RemoteProjectionPushRequest,
) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
    if request.provider() != RemoteProjectionProvider::S3 {
        return Err(RemoteProjectionProviderError::ProviderMismatch);
    }
    reject_custom_https_endpoint_without_binding(request.locator())?;
    for file in request.files() {
        push_payload(
            transport,
            credentials,
            region,
            now,
            request.locator(),
            file.path(),
            file.content().to_vec(),
        )?;
    }
    Ok(push_outcome(request.files().len()))
}

fn push_payload<T: S3Transport>(
    transport: &T,
    credentials: &S3Credentials,
    region: &str,
    now: fn() -> DateTime<Utc>,
    locator: &str,
    path: &str,
    content: Vec<u8>,
) -> Result<(), RemoteProjectionProviderError> {
    let target = s3_file_url(locator, region, path)?;
    let status = transport.put(signed_put_request(
        target,
        content,
        credentials,
        region,
        now(),
    )?)?;
    if !status.is_success() {
        return Err(RemoteProjectionProviderError::ProviderIo(format!(
            "S3 PUT {path} failed with status {}",
            status.as_u16()
        )));
    }
    Ok(())
}

fn push_outcome(uploaded_files: usize) -> RemoteProjectionPushOutcome {
    RemoteProjectionPushOutcome {
        uploaded_files,
        effects: RemoteProjectionAuthorityEffects::projection_transport(),
        provider_metadata_is_diagnostic_only: true,
    }
}
