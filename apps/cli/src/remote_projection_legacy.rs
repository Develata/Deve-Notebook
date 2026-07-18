//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!
//! Explicit B2-to-B4 transition carrier for the unpublished pull-to-workspace
//! route. It is not transport authority and is deleted by the B4 cutover.

mod outcome_contract;
mod resolved;
mod workspace_apply;

use crate::remote_projection_transport::{
    RemoteSourceAcquisition, RemoteSourceSink, SourceAcquisitionError, SourceAcquisitionRequest,
};
use anyhow::{Context, Result};
use deve_core::remote_projection::{
    RemoteProjectionAuthorityEffects, RemoteProjectionFile, RemoteProjectionProvider,
    RemoteProjectionProviderError, RemoteProjectionPullOutcome,
};
use std::io::Read;

pub(crate) use outcome_contract::ensure_projection_transport_pull_outcome_contract;
#[cfg(test)]
pub(crate) use resolved::prepared_pull_for_test;
pub(crate) use resolved::{
    LegacyPullExecutionSummary, PreparedProjectionRemotePull, apply_prepared_pull,
    finalize_prepared_pull_after_scan, prepare_pull_for_resolved_repo, scan_prepared_pull,
};
pub(crate) use workspace_apply::write_pull_files;

pub(crate) trait LegacyProjectionPullAdapter {
    fn pull_projection_files(
        &self,
        provider: RemoteProjectionProvider,
        locator: &str,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError>;
}

pub(crate) trait LegacyWebDavProjectionAdapter:
    crate::remote_projection_transport::webdav::WebDavProjectionPushAdapter
    + LegacyProjectionPullAdapter
{
}

impl<T> LegacyWebDavProjectionAdapter for T where
    T: crate::remote_projection_transport::webdav::WebDavProjectionPushAdapter
        + LegacyProjectionPullAdapter
{
}

pub(crate) trait LegacyS3ProjectionAdapter:
    crate::remote_projection_transport::s3::S3ProjectionPushAdapter + LegacyProjectionPullAdapter
{
}

impl<T> LegacyS3ProjectionAdapter for T where
    T: crate::remote_projection_transport::s3::S3ProjectionPushAdapter
        + LegacyProjectionPullAdapter
{
}

impl<T: RemoteSourceAcquisition> LegacyProjectionPullAdapter for T {
    fn pull_projection_files(
        &self,
        provider: RemoteProjectionProvider,
        locator: &str,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        if provider != self.provider() {
            return Err(RemoteProjectionProviderError::ProviderMismatch);
        }
        let request = SourceAcquisitionRequest::new(provider, locator)?;
        let mut sink = LegacyCollectingSink::default();
        self.acquire(request, &mut sink)
            .map_err(|error| match error {
                SourceAcquisitionError::Transport(error) => error,
                SourceAcquisitionError::Sink(error) => {
                    RemoteProjectionProviderError::ProviderIo(error.to_string())
                }
            })?;
        Ok(RemoteProjectionPullOutcome {
            files: sink.files,
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            overwrites_projection_workspace: true,
            external_changes_confirmation_required: true,
            provider_metadata_is_diagnostic_only: true,
        })
    }
}

#[derive(Default)]
struct LegacyCollectingSink {
    files: Vec<RemoteProjectionFile>,
}

impl RemoteSourceSink for LegacyCollectingSink {
    type Error = std::io::Error;

    fn capture(
        &mut self,
        path: &crate::remote_projection_transport::NormalizedRemotePath,
        body: &mut dyn Read,
    ) -> Result<(), Self::Error> {
        let mut content = Vec::new();
        body.read_to_end(&mut content)?;
        self.files.push(
            RemoteProjectionFile::new(path.as_str(), content)
                .expect("normalized remote path remains valid"),
        );
        Ok(())
    }
}

pub(crate) fn rollback_after_failed_scan(
    applied: workspace_apply::AppliedPullFiles,
    scan_error: &anyhow::Error,
) -> Result<()> {
    applied.rollback_after_failed_scan().with_context(|| {
        format!("remote projection pull scan failed after workspace apply: {scan_error}")
    })
}
