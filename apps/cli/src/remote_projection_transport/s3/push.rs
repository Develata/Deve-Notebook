//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract

use super::credentials::S3Credentials;
use super::provider::FailClosedS3ProjectionProvider;
use super::provider::S3ProjectionProvider;
use super::signing::signed_put_request;
use super::transport::S3Transport;
use super::url::{S3CustomEndpointUrlBinding, s3_file_url_with_binding};
use crate::remote_projection_transport::ProjectionPushSource;
use crate::remote_projection_transport::TransportCapability;
use chrono::{DateTime, Utc};
use deve_core::remote_projection::{
    RemoteProjectionAuthorityEffects, RemoteProjectionProvider, RemoteProjectionProviderError,
    RemoteProjectionPushOutcome, RemoteProjectionPushRequest,
};

pub(crate) trait S3ProjectionPushAdapter {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        locator: &str,
        source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError>;
}

impl<T: S3Transport> S3ProjectionPushAdapter for S3ProjectionProvider<T> {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        locator: &str,
        source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        if provider != RemoteProjectionProvider::S3 {
            return Err(RemoteProjectionProviderError::ProviderMismatch);
        }
        let request = RemoteProjectionPushRequest::new(provider, locator, Vec::new())?;
        let binding = self.request_binding(TransportCapability::Push, request.locator())?;
        let push_context = S3PushContext {
            transport: &self.transport,
            credentials: &binding.credentials,
            region: &binding.region,
            custom_url_binding: binding.custom_url_binding.as_ref(),
            now: self.now,
        };
        source.visit(&mut |path, content| {
            push_payload(&push_context, request.locator(), path, content)
        })?;
        Ok(push_outcome(source.file_count()))
    }
}

impl S3ProjectionPushAdapter for FailClosedS3ProjectionProvider {
    fn push_projection_files(
        &mut self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
        _source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        Err(RemoteProjectionProviderError::ProviderIo(
            "S3 push provider is unavailable in this execution path (provider_io_ready=false)"
                .into(),
        ))
    }
}

#[cfg(test)]
pub(super) fn push_request<T: S3Transport>(
    transport: &T,
    credentials: &S3Credentials,
    region: &str,
    custom_url_binding: Option<&S3CustomEndpointUrlBinding>,
    now: fn() -> DateTime<Utc>,
    request: RemoteProjectionPushRequest,
) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
    if request.provider() != RemoteProjectionProvider::S3 {
        return Err(RemoteProjectionProviderError::ProviderMismatch);
    }
    let push_context = S3PushContext {
        transport,
        credentials,
        region,
        custom_url_binding,
        now,
    };
    for file in request.files() {
        push_payload(
            &push_context,
            request.locator(),
            file.path(),
            file.content().to_vec(),
        )?;
    }
    Ok(push_outcome(request.files().len()))
}

struct S3PushContext<'a, T> {
    transport: &'a T,
    credentials: &'a S3Credentials,
    region: &'a str,
    custom_url_binding: Option<&'a S3CustomEndpointUrlBinding>,
    now: fn() -> DateTime<Utc>,
}

fn push_payload<T: S3Transport>(
    push_context: &S3PushContext<'_, T>,
    locator: &str,
    path: &str,
    content: Vec<u8>,
) -> Result<(), RemoteProjectionProviderError> {
    let target = s3_file_url_with_binding(
        locator,
        push_context.region,
        path,
        push_context.custom_url_binding,
    )?;
    let status = push_context.transport.put(signed_put_request(
        target,
        content,
        push_context.credentials,
        push_context.region,
        (push_context.now)(),
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
