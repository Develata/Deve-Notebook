//! plan_ref:
//!   - 06_backup#backup-provider-dispatch-contract

mod download;
mod transport;
mod upload;
mod url;

use super::{BackupPackDownloadOutcome, BackupPackUploadOutcome};
use deve_core::backup::{BackupLocator, BackupSecretRef};

pub(super) fn upload_webdav_pack(
    locator: &BackupLocator,
    credential_ref: &BackupSecretRef,
    object_path: &str,
    artifact_bytes: &[u8],
) -> anyhow::Result<BackupPackUploadOutcome> {
    upload::upload_webdav_pack(locator, credential_ref, object_path, artifact_bytes)
}

#[allow(dead_code)]
pub(super) fn download_webdav_pack(
    locator: &BackupLocator,
    credential_ref: &BackupSecretRef,
    object_path: &str,
    max_bytes: usize,
) -> anyhow::Result<BackupPackDownloadOutcome> {
    download::download_webdav_pack(locator, credential_ref, object_path, max_bytes)
}
