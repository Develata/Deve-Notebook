//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::super::super::s3;
use crate::remote_projection_legacy::LegacyProjectionPullAdapter;
use crate::remote_projection_transport::ProjectionPushSource;
use deve_core::remote_projection::{
    RemoteProjectionAuthorityEffects, RemoteProjectionFile, RemoteProjectionProvider,
    RemoteProjectionProviderError, RemoteProjectionPullOutcome, RemoteProjectionPushOutcome,
};

#[derive(Default)]
pub(in crate::commands::projection_remote::tests) struct RecordingS3Provider {
    pub(in crate::commands::projection_remote::tests) uploaded_paths: Vec<(String, Vec<String>)>,
}

pub(in crate::commands::projection_remote::tests) struct S3PullWritingProvider;
pub(in crate::commands::projection_remote::tests) struct S3PullFailingProvider;

impl s3::S3ProjectionPushAdapter for RecordingS3Provider {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        locator: &str,
        source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        assert_eq!(provider, RemoteProjectionProvider::S3);
        self.uploaded_paths
            .push((locator.to_string(), source_paths(source)?));
        Ok(RemoteProjectionPushOutcome {
            uploaded_files: source.file_count(),
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            provider_metadata_is_diagnostic_only: true,
        })
    }
}

impl LegacyProjectionPullAdapter for RecordingS3Provider {
    fn pull_projection_files(
        &self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        unreachable!("push-only S3 recording provider")
    }
}

impl s3::S3ProjectionPushAdapter for S3PullWritingProvider {
    fn push_projection_files(
        &mut self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
        _source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        unreachable!("pull-only S3 writing provider")
    }
}

impl LegacyProjectionPullAdapter for S3PullWritingProvider {
    fn pull_projection_files(
        &self,
        provider: RemoteProjectionProvider,
        _locator: &str,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        assert_eq!(provider, RemoteProjectionProvider::S3);
        Ok(RemoteProjectionPullOutcome {
            files: vec![RemoteProjectionFile::new("remote-s3.md", b"remote s3").expect("file")],
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            overwrites_projection_workspace: true,
            external_changes_confirmation_required: true,
            provider_metadata_is_diagnostic_only: true,
        })
    }
}

impl s3::S3ProjectionPushAdapter for S3PullFailingProvider {
    fn push_projection_files(
        &mut self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
        _source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        unreachable!("pull-only S3 failing provider")
    }
}

impl LegacyProjectionPullAdapter for S3PullFailingProvider {
    fn pull_projection_files(
        &self,
        provider: RemoteProjectionProvider,
        _locator: &str,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        assert_eq!(provider, RemoteProjectionProvider::S3);
        Err(RemoteProjectionProviderError::ProviderIo(
            "simulated S3 pull failure".into(),
        ))
    }
}

fn source_paths(
    source: &dyn ProjectionPushSource,
) -> Result<Vec<String>, RemoteProjectionProviderError> {
    let mut paths = Vec::new();
    source.visit(&mut |path, _content| {
        paths.push(path.to_string());
        Ok(())
    })?;
    Ok(paths)
}
