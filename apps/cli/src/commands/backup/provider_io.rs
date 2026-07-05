//! plan_ref:
//!   - 06_backup#backup-provider-dispatch-contract
//!   - 06_backup#backup-upload-state-machine-contract
//!   - 06_backup#backup-restore-state-machine-contract
//!
//! Backup-owned provider transfer adapters.
//!
//! This module only transfers previously sealed encrypted backup artifacts.
//! Downloaded bytes stay encrypted and must still pass manifest/hash/auth/decrypt
//! admission before any restore candidate exists. Provider metadata is diagnostic
//! only. This module does not read ledger facts, decrypt artifacts, write
//! source-control state, mutate staging, or touch Projection Workspaces.

mod credentials;
mod s3;
mod webdav;

use anyhow::bail;
use deve_core::backup::{BackupArtifactKey, BackupLocator, BackupProviderKind, BackupSecretRef};

pub(crate) const BACKUP_PACK_CONTENT_TYPE: &str = "application/vnd.deve.backup-pack+json";
pub(crate) const BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES: usize = 64 * 1024 * 1024;

pub(crate) struct BackupPackUploadRequest<'a> {
    pub locator: &'a BackupLocator,
    pub credential_ref: &'a BackupSecretRef,
    pub object_path: &'a str,
    pub artifact_bytes: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BackupPackUploadOutcome {
    pub uploaded_bytes: usize,
    pub provider_metadata_is_diagnostic_only: bool,
}

pub(crate) trait BackupPackUploader {
    fn upload_pack(
        &mut self,
        request: BackupPackUploadRequest<'_>,
    ) -> anyhow::Result<BackupPackUploadOutcome>;
}

pub(crate) struct BackupArtifactDownloadRequest<'a> {
    pub locator: &'a BackupLocator,
    pub credential_ref: &'a BackupSecretRef,
    pub object_path: &'a str,
    pub max_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackupArtifactDownloadOutcome {
    pub artifact_bytes: Vec<u8>,
    pub downloaded_bytes: usize,
    pub provider_metadata_is_diagnostic_only: bool,
}

pub(crate) trait BackupArtifactDownloader {
    fn download_artifact(
        &mut self,
        request: BackupArtifactDownloadRequest<'_>,
    ) -> anyhow::Result<BackupArtifactDownloadOutcome>;
}

pub(crate) trait BackupArtifactKeyResolver {
    fn resolve_key(&mut self, key_ref: &BackupSecretRef) -> anyhow::Result<BackupArtifactKey>;
}

pub(crate) struct RealBackupPackUploader;

impl BackupPackUploader for RealBackupPackUploader {
    fn upload_pack(
        &mut self,
        request: BackupPackUploadRequest<'_>,
    ) -> anyhow::Result<BackupPackUploadOutcome> {
        match request.locator.provider {
            BackupProviderKind::WebDavHttps => webdav::upload_webdav_pack(
                request.locator,
                request.credential_ref,
                request.object_path,
                request.artifact_bytes,
            ),
            BackupProviderKind::S3 | BackupProviderKind::S3CompatibleHttps => s3::upload_s3_pack(
                request.locator,
                request.credential_ref,
                request.object_path,
                request.artifact_bytes,
            ),
        }
    }
}

pub(crate) struct RealBackupArtifactDownloader;

impl BackupArtifactDownloader for RealBackupArtifactDownloader {
    fn download_artifact(
        &mut self,
        request: BackupArtifactDownloadRequest<'_>,
    ) -> anyhow::Result<BackupArtifactDownloadOutcome> {
        if request.max_bytes == 0 {
            bail!("backup provider download max_bytes must be greater than zero");
        }
        let max_bytes = normalize_download_limit(request.max_bytes)?;
        match request.locator.provider {
            BackupProviderKind::WebDavHttps => webdav::download_webdav_pack(
                request.locator,
                request.credential_ref,
                request.object_path,
                max_bytes,
            ),
            BackupProviderKind::S3 | BackupProviderKind::S3CompatibleHttps => s3::download_s3_pack(
                request.locator,
                request.credential_ref,
                request.object_path,
                max_bytes,
            ),
        }
    }
}

pub(crate) struct EnvBackupArtifactKeyResolver;

impl BackupArtifactKeyResolver for EnvBackupArtifactKeyResolver {
    fn resolve_key(&mut self, key_ref: &BackupSecretRef) -> anyhow::Result<BackupArtifactKey> {
        credentials::backup_artifact_key(key_ref)
    }
}

fn normalize_download_limit(requested: usize) -> anyhow::Result<usize> {
    if requested == 0 {
        bail!("backup provider download max_bytes must be greater than zero");
    }
    Ok(requested.min(BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES))
}

#[cfg(test)]
pub(crate) struct FailClosedBackupPackUploader;

#[cfg(test)]
impl BackupPackUploader for FailClosedBackupPackUploader {
    fn upload_pack(
        &mut self,
        _request: BackupPackUploadRequest<'_>,
    ) -> anyhow::Result<BackupPackUploadOutcome> {
        bail!("backup provider upload is unavailable in this execution path")
    }
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) struct FailClosedBackupArtifactDownloader;

#[cfg(test)]
#[allow(dead_code)]
impl BackupArtifactDownloader for FailClosedBackupArtifactDownloader {
    fn download_artifact(
        &mut self,
        _request: BackupArtifactDownloadRequest<'_>,
    ) -> anyhow::Result<BackupArtifactDownloadOutcome> {
        bail!("backup provider download is unavailable in this execution path")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_provider_download_limit_fails_closed_on_zero() {
        let err = normalize_download_limit(0).expect_err("zero download limit must fail closed");

        assert!(err.to_string().contains("max_bytes"));
    }

    #[test]
    fn backup_provider_download_limit_caps_to_runtime_budget() {
        let limit = normalize_download_limit(BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES + 1)
            .expect("download limit capped");

        assert_eq!(limit, BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES);
    }
}
