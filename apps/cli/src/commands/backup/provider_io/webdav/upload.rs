//! plan_ref:
//!   - 06_backup#backup-provider-dispatch-contract

use super::super::credentials::webdav_authorization_header;
use super::super::{
    BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES, BackupPackUploadOutcome, ensure_upload_readback_budget,
    verified_upload_readback_digest,
};
use super::transport::{
    ReqwestWebDavBackupTransport, WebDavBackupDownloadTransport, WebDavBackupUploadTransport,
};
use super::url::{webdav_collection_url, webdav_endpoint, webdav_object_url};
use anyhow::bail;
use deve_core::backup::{BackupLocator, BackupSecretRef};
use reqwest::{StatusCode, Url};
use std::collections::BTreeSet;

pub(super) fn upload_webdav_pack(
    locator: &BackupLocator,
    credential_ref: &BackupSecretRef,
    object_path: &str,
    artifact_bytes: &[u8],
) -> anyhow::Result<BackupPackUploadOutcome> {
    let authorization = webdav_authorization_header(credential_ref)?;
    let transport = ReqwestWebDavBackupTransport::new()?;
    upload_webdav_pack_with_transport(
        locator,
        &authorization,
        object_path,
        artifact_bytes,
        &transport,
    )
}

fn upload_webdav_pack_with_transport<
    T: WebDavBackupUploadTransport + WebDavBackupDownloadTransport,
>(
    locator: &BackupLocator,
    authorization: &str,
    object_path: &str,
    artifact_bytes: &[u8],
    transport: &T,
) -> anyhow::Result<BackupPackUploadOutcome> {
    ensure_upload_readback_budget(object_path, artifact_bytes.len())?;
    let base = webdav_endpoint(locator)?;
    let mut ensured = BTreeSet::new();
    ensure_parent_collections(transport, &base, authorization, object_path, &mut ensured)?;
    let target = webdav_object_url(&base, object_path)?;
    let status = transport.put(&target, authorization, artifact_bytes.to_vec())?;
    if !status.is_success() {
        bail!(
            "Backup WebDAV PUT {object_path} failed with status {}",
            status.as_u16()
        );
    }
    let response = transport.get(&target, authorization, BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES)?;
    if !response.status.is_success() {
        bail!(
            "Backup WebDAV readback {object_path} failed with status {}",
            response.status.as_u16()
        );
    }
    let remote_verified_payload_digest =
        verified_upload_readback_digest(object_path, artifact_bytes, &response.body)?;
    Ok(BackupPackUploadOutcome {
        uploaded_bytes: artifact_bytes.len(),
        remote_verified_payload_digest,
        provider_metadata_is_diagnostic_only: true,
    })
}

fn ensure_parent_collections<T: WebDavBackupUploadTransport>(
    transport: &T,
    base: &Url,
    authorization: &str,
    object_path: &str,
    ensured: &mut BTreeSet<String>,
) -> anyhow::Result<()> {
    let segments = object_path.split('/').collect::<Vec<_>>();
    for end in 1..segments.len() {
        let url = webdav_collection_url(base, &segments[..end])?;
        ensure_collection(transport, &url, authorization, ensured)?;
    }
    Ok(())
}

fn ensure_collection<T: WebDavBackupUploadTransport>(
    transport: &T,
    url: &Url,
    authorization: &str,
    ensured: &mut BTreeSet<String>,
) -> anyhow::Result<()> {
    if !ensured.insert(url.as_str().to_string()) {
        return Ok(());
    }
    let status = transport.mkcol(url, authorization)?;
    if status.is_success() || status == StatusCode::METHOD_NOT_ALLOWED {
        return Ok(());
    }
    bail!(
        "Backup WebDAV MKCOL {} failed with status {}",
        url,
        status.as_u16()
    )
}

#[cfg(test)]
mod tests;
