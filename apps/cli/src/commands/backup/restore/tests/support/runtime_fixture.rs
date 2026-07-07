use super::super::super::{RestoreCommandInput, restore_lines_with_runtime};
use super::artifact_fixture::ArtifactMap;
use crate::commands::backup::provider_io::{
    BackupArtifactDownloadOutcome, BackupArtifactDownloadRequest, BackupArtifactDownloader,
    BackupArtifactKeyResolver,
};
use deve_core::backup::{BackupArtifactKey, BackupSecretRef};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::commands::backup::restore::tests) struct DownloadRecord {
    pub(in crate::commands::backup::restore::tests) object_path: String,
    pub(in crate::commands::backup::restore::tests) credential_ref: String,
    pub(in crate::commands::backup::restore::tests) max_bytes: usize,
}

#[derive(Debug, Default)]
pub(in crate::commands::backup::restore::tests) struct RecordingDownloader {
    artifacts: HashMap<String, Vec<u8>>,
    metadata_is_diagnostic_only: bool,
    reported_downloaded_bytes: Option<usize>,
    pub(in crate::commands::backup::restore::tests) requests: Vec<DownloadRecord>,
}

impl RecordingDownloader {
    pub(in crate::commands::backup::restore::tests) fn new(
        artifacts: impl IntoIterator<Item = (String, Vec<u8>)>,
    ) -> Self {
        Self {
            artifacts: artifacts.into_iter().collect(),
            metadata_is_diagnostic_only: true,
            reported_downloaded_bytes: None,
            requests: Vec::new(),
        }
    }

    pub(in crate::commands::backup::restore::tests) fn with_authoritative_metadata(
        mut self,
    ) -> Self {
        self.metadata_is_diagnostic_only = false;
        self
    }

    pub(in crate::commands::backup::restore::tests) fn with_reported_downloaded_bytes(
        mut self,
        downloaded_bytes: usize,
    ) -> Self {
        self.reported_downloaded_bytes = Some(downloaded_bytes);
        self
    }
}

impl BackupArtifactDownloader for RecordingDownloader {
    fn download_artifact(
        &mut self,
        request: BackupArtifactDownloadRequest<'_>,
    ) -> anyhow::Result<BackupArtifactDownloadOutcome> {
        self.requests.push(DownloadRecord {
            object_path: request.object_path.to_string(),
            credential_ref: request.credential_ref.redacted(),
            max_bytes: request.max_bytes,
        });
        let artifact_bytes = self
            .artifacts
            .get(request.object_path)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("missing artifact {}", request.object_path))?;
        let downloaded_bytes = self
            .reported_downloaded_bytes
            .unwrap_or(artifact_bytes.len());
        Ok(BackupArtifactDownloadOutcome {
            downloaded_bytes,
            artifact_bytes,
            provider_metadata_is_diagnostic_only: self.metadata_is_diagnostic_only,
        })
    }
}

pub(in crate::commands::backup::restore::tests) struct FixedKeyResolver {
    key: BackupArtifactKey,
    pub(in crate::commands::backup::restore::tests) requests: Vec<String>,
}

impl FixedKeyResolver {
    pub(in crate::commands::backup::restore::tests) fn new(key: BackupArtifactKey) -> Self {
        Self {
            key,
            requests: Vec::new(),
        }
    }
}

impl std::fmt::Debug for FixedKeyResolver {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FixedKeyResolver")
            .field("request_count", &self.requests.len())
            .finish()
    }
}

impl BackupArtifactKeyResolver for FixedKeyResolver {
    fn resolve_key(&mut self, key_ref: &BackupSecretRef) -> anyhow::Result<BackupArtifactKey> {
        self.requests.push(key_ref.redacted());
        Ok(self.key.clone())
    }
}

pub(in crate::commands::backup::restore::tests) fn restore_with_fixture(
    command: RestoreCommandInput<'_>,
    artifacts: ArtifactMap,
    key: BackupArtifactKey,
) -> anyhow::Result<(Vec<String>, RecordingDownloader, FixedKeyResolver)> {
    let mut downloader = RecordingDownloader::new(artifacts);
    let mut key_resolver = FixedKeyResolver::new(key);
    let lines = restore_lines_with_runtime(command, &mut downloader, &mut key_resolver)?;
    Ok((lines, downloader, key_resolver))
}
