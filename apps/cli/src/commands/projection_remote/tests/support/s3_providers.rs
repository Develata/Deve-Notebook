//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::super::super::s3;
use crate::remote_projection_transport::{ProjectionPushError, ProjectionPushSource};
use deve_core::remote_projection::{
    RemoteProjectionAuthorityEffects, RemoteProjectionProvider, RemoteProjectionPushOutcome,
};

#[derive(Default)]
pub(in crate::commands::projection_remote::tests) struct RecordingS3Provider {
    pub(in crate::commands::projection_remote::tests) uploaded_paths: Vec<(String, Vec<String>)>,
}

impl s3::S3ProjectionPushAdapter for RecordingS3Provider {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        locator: &str,
        source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, ProjectionPushError> {
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

fn source_paths(source: &dyn ProjectionPushSource) -> Result<Vec<String>, ProjectionPushError> {
    let mut paths = Vec::new();
    source
        .visit(&mut |path, _content| {
            paths.push(path.to_string());
            Ok(())
        })
        .map_err(ProjectionPushError::push_failed)?;
    Ok(paths)
}
