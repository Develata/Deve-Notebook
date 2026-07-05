//! plan_ref:
//!   - 06_backup#backup-provider-dispatch-contract
//!   - 06_backup#backup-upload-state-machine-contract
//!   - 06_backup#backup-restore-state-machine-contract
//!
//! Backup-owned provider transfer adapters.
//!
//! This module only transfers previously sealed encrypted backup pack artifacts.
//! Downloaded bytes stay encrypted and must still pass manifest/hash/auth/decrypt
//! admission before any restore candidate exists. Provider metadata is diagnostic
//! only. This module does not read ledger facts, decrypt artifacts, write
//! source-control state, mutate staging, or touch Projection Workspaces.

mod credentials;
mod s3;
mod webdav;

use anyhow::bail;
use deve_core::backup::{BackupLocator, BackupProviderKind, BackupSecretRef};

pub(crate) const BACKUP_PACK_CONTENT_TYPE: &str = "application/vnd.deve.backup-pack+json";
pub(crate) const BACKUP_PACK_MAX_DOWNLOAD_BYTES: usize = 64 * 1024 * 1024;

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

pub(crate) struct BackupPackDownloadRequest<'a> {
    pub locator: &'a BackupLocator,
    pub credential_ref: &'a BackupSecretRef,
    pub object_path: &'a str,
    pub max_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackupPackDownloadOutcome {
    pub artifact_bytes: Vec<u8>,
    pub downloaded_bytes: usize,
    pub provider_metadata_is_diagnostic_only: bool,
}

pub(crate) trait BackupPackDownloader {
    fn download_pack(
        &mut self,
        request: BackupPackDownloadRequest<'_>,
    ) -> anyhow::Result<BackupPackDownloadOutcome>;
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

pub(crate) struct RealBackupPackDownloader;

impl BackupPackDownloader for RealBackupPackDownloader {
    fn download_pack(
        &mut self,
        request: BackupPackDownloadRequest<'_>,
    ) -> anyhow::Result<BackupPackDownloadOutcome> {
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

fn normalize_download_limit(requested: usize) -> anyhow::Result<usize> {
    if requested == 0 {
        bail!("backup provider download max_bytes must be greater than zero");
    }
    Ok(requested.min(BACKUP_PACK_MAX_DOWNLOAD_BYTES))
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
pub(crate) struct FailClosedBackupPackDownloader;

#[cfg(test)]
#[allow(dead_code)]
impl BackupPackDownloader for FailClosedBackupPackDownloader {
    fn download_pack(
        &mut self,
        _request: BackupPackDownloadRequest<'_>,
    ) -> anyhow::Result<BackupPackDownloadOutcome> {
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
        let limit = normalize_download_limit(BACKUP_PACK_MAX_DOWNLOAD_BYTES + 1)
            .expect("download limit capped");

        assert_eq!(limit, BACKUP_PACK_MAX_DOWNLOAD_BYTES);
    }
}
