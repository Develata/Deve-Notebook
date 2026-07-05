//! plan_ref:
//!   - 06_backup#backup-provider-dispatch-contract

use super::super::BackupPackUploadOutcome;
use super::super::credentials::webdav_authorization_header;
use super::transport::{ReqwestWebDavBackupTransport, WebDavBackupUploadTransport};
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

fn upload_webdav_pack_with_transport<T: WebDavBackupUploadTransport>(
    locator: &BackupLocator,
    authorization: &str,
    object_path: &str,
    artifact_bytes: &[u8],
    transport: &T,
) -> anyhow::Result<BackupPackUploadOutcome> {
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
    Ok(BackupPackUploadOutcome {
        uploaded_bytes: artifact_bytes.len(),
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
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingWebDavTransport {
        mkcol_urls: Mutex<Vec<String>>,
        put_calls: Mutex<Vec<(String, Vec<u8>)>>,
        put_status: Mutex<Option<StatusCode>>,
    }

    impl WebDavBackupUploadTransport for RecordingWebDavTransport {
        fn mkcol(&self, url: &Url, _authorization: &str) -> anyhow::Result<StatusCode> {
            self.mkcol_urls
                .lock()
                .expect("mkcol urls")
                .push(url.as_str().to_string());
            Ok(StatusCode::CREATED)
        }

        fn put(
            &self,
            url: &Url,
            _authorization: &str,
            body: Vec<u8>,
        ) -> anyhow::Result<StatusCode> {
            self.put_calls
                .lock()
                .expect("put calls")
                .push((url.as_str().to_string(), body));
            if let Some(status) = self.put_status.lock().expect("put status").take() {
                return Ok(status);
            }
            Ok(StatusCode::CREATED)
        }
    }

    #[test]
    fn webdav_upload_puts_backup_pack_bytes_under_branch_pack_path() {
        let locator = BackupLocator::parse("webdav+https://dav.example.com/deve").unwrap();
        let transport = RecordingWebDavTransport::default();
        let outcome = upload_webdav_pack_with_transport(
            &locator,
            "Bearer token",
            "deve/branches/writer-1/packs/000001.pack.enc",
            b"encrypted-json",
            &transport,
        )
        .unwrap();

        assert_eq!(outcome.uploaded_bytes, "encrypted-json".len());
        assert!(outcome.provider_metadata_is_diagnostic_only);
        let put_calls = transport.put_calls.lock().expect("put calls");
        assert_eq!(put_calls.len(), 1);
        assert_eq!(
            put_calls[0].0,
            "https://dav.example.com/deve/branches/writer-1/packs/000001.pack.enc"
        );
        assert_eq!(put_calls[0].1, b"encrypted-json");
    }

    #[test]
    fn webdav_upload_rejects_non_success_status() {
        let locator = BackupLocator::parse("webdav+https://dav.example.com/deve").unwrap();
        let transport = RecordingWebDavTransport::default();
        *transport.put_status.lock().expect("put status") = Some(StatusCode::FORBIDDEN);

        let err = upload_webdav_pack_with_transport(
            &locator,
            "Bearer token",
            "deve/branches/writer-1/packs/000001.pack.enc",
            b"encrypted-json",
            &transport,
        )
        .expect_err("non-2xx status must fail before Uploaded state");

        assert!(err.to_string().contains("403"));
    }
}
