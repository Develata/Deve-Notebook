//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract

use super::credentials::S3Credentials;
use super::list::discover_remote_markdown_files;
use super::provider::S3ProjectionProvider;
use super::signing::signed_get_request;
use super::transport::S3Transport;
use super::url::{S3CustomEndpointUrlBinding, s3_file_url_with_binding};
use crate::remote_projection_transport::{
    RemoteSourceAcquisition, RemoteSourceSink, SourceAcquisitionError, SourceAcquisitionOutcome,
    SourceAcquisitionRequest, TransportCapability,
};
use chrono::{DateTime, Utc};
use deve_core::remote_projection::{RemoteProjectionProvider, RemoteProjectionProviderError};
use reqwest::StatusCode;

pub(super) const MAX_SOURCE_FILE_BYTES: usize = 4 * 1024 * 1024;
pub(super) const MAX_SOURCE_TOTAL_BYTES: usize = 64 * 1024 * 1024;

impl<T: S3Transport> RemoteSourceAcquisition for S3ProjectionProvider<T> {
    fn acquire<S: RemoteSourceSink>(
        &self,
        request: SourceAcquisitionRequest,
        sink: &mut S,
    ) -> Result<SourceAcquisitionOutcome, SourceAcquisitionError<S::Error>> {
        if request.provider() != RemoteProjectionProvider::S3 {
            return Err(RemoteProjectionProviderError::ProviderMismatch.into());
        }
        let binding =
            self.request_binding(TransportCapability::SourceAcquisition, request.locator())?;
        acquire_source(
            &self.transport,
            &binding.credentials,
            &binding.region,
            binding.custom_url_binding.as_ref(),
            self.now,
            request,
            sink,
        )
    }
}

pub(super) fn acquire_source<T: S3Transport, S: RemoteSourceSink>(
    transport: &T,
    credentials: &S3Credentials,
    region: &str,
    custom_url_binding: Option<&S3CustomEndpointUrlBinding>,
    now: fn() -> DateTime<Utc>,
    request: SourceAcquisitionRequest,
    sink: &mut S,
) -> Result<SourceAcquisitionOutcome, SourceAcquisitionError<S::Error>> {
    if request.provider() != RemoteProjectionProvider::S3 {
        return Err(RemoteProjectionProviderError::ProviderMismatch.into());
    }
    let paths = discover_remote_markdown_files(
        transport,
        credentials,
        region,
        custom_url_binding,
        now,
        request.locator(),
    )?;
    let file_count = paths.len();
    let mut total_bytes = 0usize;
    for path in paths {
        let target =
            s3_file_url_with_binding(request.locator(), region, path.as_str(), custom_url_binding)?;
        let mut response = transport.get_stream(signed_get_request(
            target,
            credentials,
            region,
            now(),
            MAX_SOURCE_FILE_BYTES,
        )?)?;
        if response.status != StatusCode::OK {
            return Err(RemoteProjectionProviderError::ProviderIo(format!(
                "S3 GET {} failed with status {}",
                path.as_str(),
                response.status.as_u16()
            ))
            .into());
        }
        let bytes = super::super::body::capture_bounded_body(
            "S3",
            &path,
            response.body.as_mut(),
            super::super::body::BodyCaptureBudget {
                max_file_bytes: MAX_SOURCE_FILE_BYTES,
                remaining_total_bytes: MAX_SOURCE_TOTAL_BYTES.saturating_sub(total_bytes),
                max_total_bytes: MAX_SOURCE_TOTAL_BYTES,
            },
            sink,
        )?;
        total_bytes = total_bytes
            .checked_add(bytes)
            .ok_or_else(|| source_budget_error("total downloaded bytes overflow"))?;
        if total_bytes > MAX_SOURCE_TOTAL_BYTES {
            return Err(source_budget_error(format!(
                "S3 source acquisition exceeds total byte budget of {MAX_SOURCE_TOTAL_BYTES}"
            ))
            .into());
        }
    }
    Ok(SourceAcquisitionOutcome {
        files: file_count,
        bytes: total_bytes,
    })
}

fn source_budget_error(message: impl Into<String>) -> RemoteProjectionProviderError {
    RemoteProjectionProviderError::ProviderIo(message.into())
}
