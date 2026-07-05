//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::credentials::S3Credentials;
use super::list::discover_remote_markdown_files;
use super::provider::{FailClosedS3ProjectionProvider, S3ProjectionProvider};
use super::signing::signed_get_request;
use super::transport::S3Transport;
use super::url::s3_file_url;
use crate::commands::projection_remote::workspace_apply::write_pull_files;
use chrono::{DateTime, Utc};
use deve_core::remote_projection::{
    RemoteProjectionAuthorityEffects, RemoteProjectionFile, RemoteProjectionProvider,
    RemoteProjectionProviderError, RemoteProjectionPullOutcome, RemoteProjectionPullRequest,
};
use std::path::Path;

pub(super) const MAX_PULL_FILE_BYTES: usize = 4 * 1024 * 1024;
const MAX_PULL_TOTAL_BYTES: usize = 64 * 1024 * 1024;

pub(crate) trait S3ProjectionPullAdapter {
    fn pull_projection_files(
        &self,
        provider: RemoteProjectionProvider,
        locator: &str,
        workspace: &Path,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError>;
}

impl<T: S3Transport> S3ProjectionPullAdapter for S3ProjectionProvider<T> {
    fn pull_projection_files(
        &self,
        provider: RemoteProjectionProvider,
        locator: &str,
        workspace: &Path,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        if provider != RemoteProjectionProvider::S3 {
            return Err(RemoteProjectionProviderError::ProviderMismatch);
        }
        let request = RemoteProjectionPullRequest::new(provider, locator)?;
        let credentials = self.credentials.resolve()?;
        let region = self.region.resolve()?;
        let outcome = pull_request(&self.transport, &credentials, &region, self.now, request)?;
        write_pull_files(workspace, &outcome.files)?;
        Ok(outcome)
    }
}

impl S3ProjectionPullAdapter for FailClosedS3ProjectionProvider {
    fn pull_projection_files(
        &self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
        _workspace: &Path,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        Err(RemoteProjectionProviderError::ProviderIo(
            "S3 pull provider is unavailable in this execution path (provider_io_ready=false)"
                .into(),
        ))
    }
}

pub(super) fn pull_request<T: S3Transport>(
    transport: &T,
    credentials: &S3Credentials,
    region: &str,
    now: fn() -> DateTime<Utc>,
    request: RemoteProjectionPullRequest,
) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
    if request.provider() != RemoteProjectionProvider::S3 {
        return Err(RemoteProjectionProviderError::ProviderMismatch);
    }
    let paths =
        discover_remote_markdown_files(transport, credentials, region, now, request.locator())?;
    let mut files = Vec::new();
    let mut total_bytes = 0usize;
    for path in paths {
        let target = s3_file_url(request.locator(), region, &path)?;
        let response = transport.get(signed_get_request(
            target,
            credentials,
            region,
            now(),
            MAX_PULL_FILE_BYTES,
        )?)?;
        if !response.status.is_success() {
            return Err(RemoteProjectionProviderError::ProviderIo(format!(
                "S3 GET {path} failed with status {}",
                response.status.as_u16()
            )));
        }
        total_bytes = total_bytes
            .checked_add(response.body.len())
            .ok_or_else(|| pull_budget_error("total downloaded bytes overflow"))?;
        if total_bytes > MAX_PULL_TOTAL_BYTES {
            return Err(pull_budget_error(format!(
                "S3 pull exceeds total byte budget of {MAX_PULL_TOTAL_BYTES}"
            )));
        }
        files.push(RemoteProjectionFile::new(&path, response.body)?);
    }
    Ok(RemoteProjectionPullOutcome {
        files,
        effects: RemoteProjectionAuthorityEffects::projection_transport(),
        overwrites_projection_workspace: true,
        external_changes_confirmation_required: true,
        provider_metadata_is_diagnostic_only: true,
    })
}

fn pull_budget_error(message: impl Into<String>) -> RemoteProjectionProviderError {
    RemoteProjectionProviderError::ProviderIo(message.into())
}
