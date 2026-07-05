//! plan_ref:
//!   - 06_backup#backup-provider-dispatch-contract
//!   - 06_backup#backup-upload-state-machine-contract
//!
//! Backup-owned provider upload adapters.
//!
//! This module only uploads previously sealed encrypted backup pack artifacts.
//! It does not read ledger facts, decrypt artifacts, write source-control state,
//! mutate staging, or touch Projection Workspaces.

mod credentials;
mod s3;
mod webdav;

#[cfg(test)]
use anyhow::bail;
use deve_core::backup::{BackupLocator, BackupProviderKind, BackupSecretRef};

pub(crate) const BACKUP_PACK_CONTENT_TYPE: &str = "application/vnd.deve.backup-pack+json";

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
