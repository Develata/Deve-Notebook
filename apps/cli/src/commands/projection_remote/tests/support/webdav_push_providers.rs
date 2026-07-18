//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::super::super::webdav;
use crate::remote_projection_transport::{ProjectionPushError, ProjectionPushSource};
use deve_core::remote_projection::{
    RemoteProjectionAuthorityEffects, RemoteProjectionProvider, RemoteProjectionProviderError,
    RemoteProjectionPushOutcome,
};

#[derive(Default)]
pub(in crate::commands::projection_remote::tests) struct RecordingProvider {
    pub(in crate::commands::projection_remote::tests) uploaded_paths: Vec<(String, Vec<String>)>,
}

pub(in crate::commands::projection_remote::tests) struct FailingProvider;
pub(in crate::commands::projection_remote::tests) struct AuthorityEffectPushProvider;
pub(in crate::commands::projection_remote::tests) struct AuthoritativeMetadataPushProvider;

impl webdav::WebDavProjectionPushAdapter for FailingProvider {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        _locator: &str,
        source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, ProjectionPushError> {
        assert_eq!(provider, RemoteProjectionProvider::WebDav);
        assert_eq!(source.file_count(), 1);
        Err(ProjectionPushError::push_failed(
            RemoteProjectionProviderError::ProviderIo("simulated WebDAV failure".into()),
        ))
    }
}

impl webdav::WebDavProjectionPushAdapter for RecordingProvider {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        locator: &str,
        source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, ProjectionPushError> {
        assert_eq!(provider, RemoteProjectionProvider::WebDav);
        self.uploaded_paths
            .push((locator.to_string(), source_paths(source)?));
        Ok(RemoteProjectionPushOutcome {
            uploaded_files: source.file_count(),
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            provider_metadata_is_diagnostic_only: true,
        })
    }
}

impl webdav::WebDavProjectionPushAdapter for AuthorityEffectPushProvider {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        _locator: &str,
        source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, ProjectionPushError> {
        assert_eq!(provider, RemoteProjectionProvider::WebDav);
        Ok(RemoteProjectionPushOutcome {
            uploaded_files: source.file_count(),
            effects: RemoteProjectionAuthorityEffects {
                writes_ledger: true,
                writes_source_control_staging: false,
                writes_commit_anchor: false,
                writes_git_main_mirror: false,
                confirms_external_changes: false,
            },
            provider_metadata_is_diagnostic_only: true,
        })
    }
}

impl webdav::WebDavProjectionPushAdapter for AuthoritativeMetadataPushProvider {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        _locator: &str,
        source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, ProjectionPushError> {
        assert_eq!(provider, RemoteProjectionProvider::WebDav);
        Ok(RemoteProjectionPushOutcome {
            uploaded_files: source.file_count(),
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            provider_metadata_is_diagnostic_only: false,
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
