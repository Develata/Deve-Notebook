//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::super::*;
use deve_core::remote_projection::{
    RemoteProjectionAuthorityEffects, RemoteProjectionFile, RemoteProjectionProviderError,
    RemoteProjectionPullOutcome, RemoteProjectionPushOutcome,
};
use std::path::PathBuf;

#[derive(Default)]
pub(super) struct RecordingProvider {
    pub(super) uploaded_paths: Vec<(String, Vec<String>)>,
}

#[derive(Default)]
pub(super) struct RecordingS3Provider {
    pub(super) uploaded_paths: Vec<(String, Vec<String>)>,
}

pub(super) struct FailingProvider;
pub(super) struct AuthorityEffectPushProvider;
pub(super) struct AuthoritativeMetadataPushProvider;
pub(super) struct PullWritingProvider;
pub(super) struct PullFailingProvider;
pub(super) struct PullWithoutWorkspaceOverwriteProvider;
pub(super) struct PullWithoutExternalChangesProvider;
pub(super) struct S3PullWritingProvider;
pub(super) struct S3PullFailingProvider;

impl webdav::WebDavProjectionPushAdapter for FailingProvider {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        _locator: &str,
        files: &[webdav::MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        assert_eq!(provider, RemoteProjectionProvider::WebDav);
        assert_eq!(files.len(), 1);
        Err(RemoteProjectionProviderError::ProviderIo(
            "simulated WebDAV failure".into(),
        ))
    }
}

impl webdav::WebDavProjectionPullAdapter for FailingProvider {
    fn pull_projection_files(
        &self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        unreachable!("push-only failing provider")
    }
}

impl webdav::WebDavProjectionPushAdapter for RecordingProvider {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        locator: &str,
        files: &[webdav::MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        assert_eq!(provider, RemoteProjectionProvider::WebDav);
        self.uploaded_paths.push((
            locator.to_string(),
            files.iter().map(|file| file.path().to_string()).collect(),
        ));
        Ok(RemoteProjectionPushOutcome {
            uploaded_files: files.len(),
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            provider_metadata_is_diagnostic_only: true,
        })
    }
}

impl webdav::WebDavProjectionPullAdapter for RecordingProvider {
    fn pull_projection_files(
        &self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        unreachable!("push-only recording provider")
    }
}

impl webdav::WebDavProjectionPushAdapter for AuthorityEffectPushProvider {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        _locator: &str,
        files: &[webdav::MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        assert_eq!(provider, RemoteProjectionProvider::WebDav);
        Ok(RemoteProjectionPushOutcome {
            uploaded_files: files.len(),
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

impl webdav::WebDavProjectionPullAdapter for AuthorityEffectPushProvider {
    fn pull_projection_files(
        &self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        unreachable!("push-only authority-effect provider")
    }
}

impl webdav::WebDavProjectionPushAdapter for AuthoritativeMetadataPushProvider {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        _locator: &str,
        files: &[webdav::MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        assert_eq!(provider, RemoteProjectionProvider::WebDav);
        Ok(RemoteProjectionPushOutcome {
            uploaded_files: files.len(),
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            provider_metadata_is_diagnostic_only: false,
        })
    }
}

impl webdav::WebDavProjectionPullAdapter for AuthoritativeMetadataPushProvider {
    fn pull_projection_files(
        &self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        unreachable!("push-only authoritative-metadata provider")
    }
}

impl s3::S3ProjectionPushAdapter for RecordingS3Provider {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        locator: &str,
        files: &[collect::MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        assert_eq!(provider, RemoteProjectionProvider::S3);
        self.uploaded_paths.push((
            locator.to_string(),
            files.iter().map(|file| file.path().to_string()).collect(),
        ));
        Ok(RemoteProjectionPushOutcome {
            uploaded_files: files.len(),
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            provider_metadata_is_diagnostic_only: true,
        })
    }
}

impl s3::S3ProjectionPullAdapter for RecordingS3Provider {
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
        _files: &[collect::MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        unreachable!("pull-only S3 writing provider")
    }
}

impl s3::S3ProjectionPullAdapter for S3PullWritingProvider {
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
        _files: &[collect::MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        unreachable!("pull-only S3 failing provider")
    }
}

impl s3::S3ProjectionPullAdapter for S3PullFailingProvider {
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

impl webdav::WebDavProjectionPushAdapter for PullWritingProvider {
    fn push_projection_files(
        &mut self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
        _files: &[webdav::MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        unreachable!("pull-only writing provider")
    }
}

impl webdav::WebDavProjectionPullAdapter for PullWritingProvider {
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
        _files: &[webdav::MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        unreachable!("pull-only workspace-overwrite-contract provider")
    }
}

impl webdav::WebDavProjectionPullAdapter for PullWithoutWorkspaceOverwriteProvider {
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

impl webdav::WebDavProjectionPushAdapter for PullWithoutExternalChangesProvider {
    fn push_projection_files(
        &mut self,
        _provider: RemoteProjectionProvider,
        _locator: &str,
        _files: &[webdav::MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        unreachable!("pull-only external-changes-contract provider")
    }
}

impl webdav::WebDavProjectionPullAdapter for PullWithoutExternalChangesProvider {
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
        _files: &[webdav::MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        unreachable!("pull-only failing provider")
    }
}

impl webdav::WebDavProjectionPullAdapter for PullFailingProvider {
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

pub(super) struct ProjectionRemoteHarness {
    _dir: tempfile::TempDir,
    root: PathBuf,
    pub(super) workspace: PathBuf,
}

impl ProjectionRemoteHarness {
    pub(super) fn ledger_dir(&self) -> PathBuf {
        self.root.join("ledger")
    }
}

pub(super) fn initialized_default_repo() -> ProjectionRemoteHarness {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().to_path_buf();
    crate::commands::init::run(
        &root.join("ledger"),
        "default",
        &root.join("notes"),
        root.clone(),
        8,
        None,
        None,
    )
    .expect("init");
    let workspace = std::fs::read_dir(root.join("notes"))
        .expect("notes dir")
        .map(|entry| entry.expect("workspace entry").path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("default--"))
        })
        .expect("default workspace");

    ProjectionRemoteHarness {
        _dir: dir,
        root,
        workspace,
    }
}

pub(super) fn s3_pull_action() -> ProjectionRemoteAction {
    ProjectionRemoteAction::S3 {
        action: ProjectionRemoteDirectionAction::Pull {
            repo: Some("default".into()),
            locator: "s3://bucket/notebooks/main".into(),
        },
    }
}

pub(super) fn webdav_pull_action() -> ProjectionRemoteAction {
    ProjectionRemoteAction::Webdav {
        action: ProjectionRemoteDirectionAction::Pull {
            repo: Some("default".into()),
            locator: "webdav+https://dav.example.com/notebooks/main".into(),
        },
    }
}

pub(super) fn webdav_push_action() -> ProjectionRemoteAction {
    ProjectionRemoteAction::Webdav {
        action: ProjectionRemoteDirectionAction::Push {
            repo: Some("default".into()),
            locator: "webdav+https://dav.example.com/notebooks/main".into(),
        },
    }
}

pub(super) fn s3_push_action() -> ProjectionRemoteAction {
    ProjectionRemoteAction::S3 {
        action: ProjectionRemoteDirectionAction::Push {
            repo: Some("default".into()),
            locator: "s3://bucket/notebooks/main".into(),
        },
    }
}
