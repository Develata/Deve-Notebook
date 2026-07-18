//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::super::super::webdav;
use crate::remote_projection_legacy::LegacyProjectionPullAdapter;
use crate::remote_projection_transport::ProjectionPushSource;
use deve_core::remote_projection::{
    RemoteProjectionAuthorityEffects, RemoteProjectionFile, RemoteProjectionProvider,
    RemoteProjectionProviderError, RemoteProjectionPullOutcome, RemoteProjectionPushOutcome,
};

pub(in crate::commands::projection_remote::tests) struct PullWritingProvider;
pub(in crate::commands::projection_remote::tests) struct PullFailingProvider;
pub(in crate::commands::projection_remote::tests) struct PullDuplicatePathProvider;
pub(in crate::commands::projection_remote::tests) struct PullWithoutWorkspaceOverwriteProvider;
pub(in crate::commands::projection_remote::tests) struct PullWithoutExternalChangesProvider;

impl webdav::WebDavProjectionPushAdapter for PullWritingProvider {
    fn push_projection_files(
        &mut self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
        _source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        unreachable!("pull-only writing provider")
    }
}

impl LegacyProjectionPullAdapter for PullWritingProvider {
    fn pull_projection_files(
        &self,
        provider: RemoteProjectionProvider,
        _locator: &str,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        assert_eq!(provider, RemoteProjectionProvider::WebDav);
        Ok(RemoteProjectionPullOutcome {
            files: vec![RemoteProjectionFile::new("remote.md", b"remote").expect("file")],
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            overwrites_projection_workspace: true,
            external_changes_confirmation_required: true,
            provider_metadata_is_diagnostic_only: true,
        })
    }
}

impl webdav::WebDavProjectionPushAdapter for PullWithoutWorkspaceOverwriteProvider {
    fn push_projection_files(
        &mut self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
        _source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        unreachable!("pull-only workspace-overwrite-contract provider")
    }
}

impl LegacyProjectionPullAdapter for PullWithoutWorkspaceOverwriteProvider {
    fn pull_projection_files(
        &self,
        provider: RemoteProjectionProvider,
        _locator: &str,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        assert_eq!(provider, RemoteProjectionProvider::WebDav);
        Ok(RemoteProjectionPullOutcome {
            files: vec![
                RemoteProjectionFile::new("remote-no-overwrite.md", b"remote").expect("file"),
            ],
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            overwrites_projection_workspace: false,
            external_changes_confirmation_required: true,
            provider_metadata_is_diagnostic_only: true,
        })
    }
}

impl webdav::WebDavProjectionPushAdapter for PullDuplicatePathProvider {
    fn push_projection_files(
        &mut self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
        _source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        unreachable!("pull-only duplicate-path-contract provider")
    }
}

impl LegacyProjectionPullAdapter for PullDuplicatePathProvider {
    fn pull_projection_files(
        &self,
        provider: RemoteProjectionProvider,
        _locator: &str,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        assert_eq!(provider, RemoteProjectionProvider::WebDav);
        Ok(RemoteProjectionPullOutcome {
            files: vec![
                RemoteProjectionFile::new("remote-duplicate.md", b"first").expect("first"),
                RemoteProjectionFile::new("remote-duplicate.md", b"second").expect("second"),
            ],
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            overwrites_projection_workspace: true,
            external_changes_confirmation_required: true,
            provider_metadata_is_diagnostic_only: true,
        })
    }
}

impl webdav::WebDavProjectionPushAdapter for PullWithoutExternalChangesProvider {
    fn push_projection_files(
        &mut self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
        _source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        unreachable!("pull-only external-changes-contract provider")
    }
}

impl LegacyProjectionPullAdapter for PullWithoutExternalChangesProvider {
    fn pull_projection_files(
        &self,
        provider: RemoteProjectionProvider,
        _locator: &str,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        assert_eq!(provider, RemoteProjectionProvider::WebDav);
        Ok(RemoteProjectionPullOutcome {
            files: vec![
                RemoteProjectionFile::new("remote-unconfirmed.md", b"remote").expect("file"),
            ],
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            overwrites_projection_workspace: true,
            external_changes_confirmation_required: false,
            provider_metadata_is_diagnostic_only: true,
        })
    }
}

impl webdav::WebDavProjectionPushAdapter for PullFailingProvider {
    fn push_projection_files(
        &mut self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
        _source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        unreachable!("pull-only failing provider")
    }
}

impl LegacyProjectionPullAdapter for PullFailingProvider {
    fn pull_projection_files(
        &self,
        provider: RemoteProjectionProvider,
        _locator: &str,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        assert_eq!(provider, RemoteProjectionProvider::WebDav);
        Err(RemoteProjectionProviderError::ProviderIo(
            "simulated WebDAV pull failure".into(),
        ))
    }
}
