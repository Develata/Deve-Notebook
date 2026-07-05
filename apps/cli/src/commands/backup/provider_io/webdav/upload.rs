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
mod tests {
    use super::super::transport::WebDavBackupHttpResponse;
    use super::*;
    use std::sync::Mutex;

    struct RecordingWebDavTransport {
        mkcol_urls: Mutex<Vec<String>>,
        put_calls: Mutex<Vec<(String, Vec<u8>)>>,
        get_calls: Mutex<Vec<(String, usize)>>,
        put_status: Mutex<Option<StatusCode>>,
        get_response: Mutex<Option<WebDavBackupHttpResponse>>,
    }

    impl RecordingWebDavTransport {
        fn with_readback(body: Vec<u8>) -> Self {
            Self::with_readback_response(StatusCode::OK, body)
        }

        fn with_readback_response(status: StatusCode, body: Vec<u8>) -> Self {
            Self {
                mkcol_urls: Mutex::new(Vec::new()),
                put_calls: Mutex::new(Vec::new()),
                get_calls: Mutex::new(Vec::new()),
                put_status: Mutex::new(None),
                get_response: Mutex::new(Some(WebDavBackupHttpResponse { status, body })),
            }
        }

        fn failing_put(status: StatusCode) -> Self {
            Self {
                mkcol_urls: Mutex::new(Vec::new()),
                put_calls: Mutex::new(Vec::new()),
                get_calls: Mutex::new(Vec::new()),
                put_status: Mutex::new(Some(status)),
                get_response: Mutex::new(None),
            }
        }
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

    impl WebDavBackupDownloadTransport for RecordingWebDavTransport {
        fn get(
            &self,
            url: &Url,
            _authorization: &str,
            max_body_bytes: usize,
        ) -> anyhow::Result<WebDavBackupHttpResponse> {
            self.get_calls
                .lock()
                .expect("get calls")
                .push((url.as_str().to_string(), max_body_bytes));
            Ok(self
                .get_response
                .lock()
                .expect("get response")
                .take()
                .expect("get response configured"))
        }
    }

    #[test]
    fn webdav_upload_remote_verifies_readback_bytes() {
        let locator = BackupLocator::parse("webdav+https://dav.example.com/deve").unwrap();
        let transport = RecordingWebDavTransport::with_readback(b"encrypted-json".to_vec());
        let outcome = upload_webdav_pack_with_transport(
            &locator,
            "Bearer token",
            "deve/branches/writer-1/packs/000001.pack.enc",
            b"encrypted-json",
            &transport,
        )
        .unwrap();

        assert_eq!(outcome.uploaded_bytes, "encrypted-json".len());
        assert_eq!(
            outcome.remote_verified_payload_digest.hex,
            "ab84e44ea5e0c43b420ce52218c2305b2457fac68327fcae73f9151770abbabc"
        );
        assert!(outcome.provider_metadata_is_diagnostic_only);
        let put_calls = transport.put_calls.lock().expect("put calls");
        assert_eq!(put_calls.len(), 1);
        assert_eq!(
            put_calls[0].0,
            "https://dav.example.com/deve/branches/writer-1/packs/000001.pack.enc"
        );
        assert_eq!(put_calls[0].1, b"encrypted-json");
        let get_calls = transport.get_calls.lock().expect("get calls");
        assert_eq!(
            get_calls.as_slice(),
            &[(
                "https://dav.example.com/deve/branches/writer-1/packs/000001.pack.enc".to_string(),
                BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES
            )]
        );
    }

    #[test]
    fn webdav_upload_rejects_readback_mismatch() {
        let locator = BackupLocator::parse("webdav+https://dav.example.com/deve").unwrap();
        let transport = RecordingWebDavTransport::with_readback(b"tampered-json".to_vec());

        let err = upload_webdav_pack_with_transport(
            &locator,
            "Bearer token",
            "deve/branches/writer-1/packs/000001.pack.enc",
            b"encrypted-json",
            &transport,
        )
        .expect_err("remote readback mismatch must fail closed");

        assert!(err.to_string().contains("readback"));
    }

    #[test]
    fn webdav_upload_rejects_readback_non_success_status() {
        let locator = BackupLocator::parse("webdav+https://dav.example.com/deve").unwrap();
        let transport =
            RecordingWebDavTransport::with_readback_response(StatusCode::NOT_FOUND, Vec::new());

        let err = upload_webdav_pack_with_transport(
            &locator,
            "Bearer token",
            "deve/branches/writer-1/packs/000001.pack.enc",
            b"encrypted-json",
            &transport,
        )
        .expect_err("remote readback non-success status must fail closed");

        assert!(err.to_string().contains("readback"));
        assert!(err.to_string().contains("404"));
    }

    #[test]
    fn webdav_upload_rejects_non_success_status() {
        let locator = BackupLocator::parse("webdav+https://dav.example.com/deve").unwrap();
        let transport = RecordingWebDavTransport::failing_put(StatusCode::FORBIDDEN);

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
