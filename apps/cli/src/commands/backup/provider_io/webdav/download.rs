//! plan_ref:
//!   - 06_backup#backup-provider-dispatch-contract
//!   - 06_backup#backup-restore-state-machine-contract

use super::super::BackupArtifactDownloadOutcome;
use super::super::credentials::webdav_authorization_header;
use super::transport::{ReqwestWebDavBackupTransport, WebDavBackupDownloadTransport};
use super::url::{webdav_endpoint, webdav_object_url};
use anyhow::bail;
use deve_core::backup::{BackupLocator, BackupSecretRef};

#[allow(dead_code)]
pub(super) fn download_webdav_pack(
    locator: &BackupLocator,
    credential_ref: &BackupSecretRef,
    object_path: &str,
    max_bytes: usize,
) -> anyhow::Result<BackupArtifactDownloadOutcome> {
    let authorization = webdav_authorization_header(credential_ref)?;
    let transport = ReqwestWebDavBackupTransport::new()?;
    download_webdav_pack_with_transport(locator, &authorization, object_path, max_bytes, &transport)
}

#[allow(dead_code)]
fn download_webdav_pack_with_transport<T: WebDavBackupDownloadTransport>(
    locator: &BackupLocator,
    authorization: &str,
    object_path: &str,
    max_bytes: usize,
    transport: &T,
) -> anyhow::Result<BackupArtifactDownloadOutcome> {
    let base = webdav_endpoint(locator)?;
    let target = webdav_object_url(&base, object_path)?;
    let response = transport.get(&target, authorization, max_bytes)?;
    if !response.status.is_success() {
        bail!(
            "Backup WebDAV GET {object_path} failed with status {}",
            response.status.as_u16()
        );
    }
    if response.body.len() > max_bytes {
        bail!("Backup WebDAV GET {object_path} exceeded max download bytes");
    }
    let downloaded_bytes = response.body.len();
    Ok(BackupArtifactDownloadOutcome {
        artifact_bytes: response.body,
        downloaded_bytes,
        provider_metadata_is_diagnostic_only: true,
    })
}

#[cfg(test)]
mod tests {
    use super::super::transport::WebDavBackupHttpResponse;
    use super::*;
    use reqwest::{StatusCode, Url};
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingWebDavTransport {
        get_calls: Mutex<Vec<(String, usize)>>,
        response: Mutex<Option<WebDavBackupHttpResponse>>,
    }

    impl RecordingWebDavTransport {
        fn with_response(status: StatusCode, body: Vec<u8>) -> Self {
            Self {
                get_calls: Mutex::new(Vec::new()),
                response: Mutex::new(Some(WebDavBackupHttpResponse { status, body })),
            }
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
                .response
                .lock()
                .expect("response")
                .take()
                .expect("response configured"))
        }
    }

    #[test]
    fn backup_provider_download_returns_encrypted_bytes_only() {
        let locator = BackupLocator::parse("webdav+https://dav.example.com/deve").unwrap();
        let transport =
            RecordingWebDavTransport::with_response(StatusCode::OK, b"encrypted-json".to_vec());

        let outcome = download_webdav_pack_with_transport(
            &locator,
            "Bearer token",
            "deve/branches/writer-1/packs/000001.pack.enc",
            1024,
            &transport,
        )
        .unwrap();

        assert_eq!(outcome.artifact_bytes, b"encrypted-json");
        assert_eq!(outcome.downloaded_bytes, "encrypted-json".len());
        assert!(outcome.provider_metadata_is_diagnostic_only);
        let calls = transport.get_calls.lock().expect("get calls");
        assert_eq!(
            calls.as_slice(),
            &[(
                "https://dav.example.com/deve/branches/writer-1/packs/000001.pack.enc".to_string(),
                1024
            )]
        );
    }

    #[test]
    fn backup_provider_download_accepts_branch_manifest_artifact_path() {
        let locator = BackupLocator::parse("webdav+https://dav.example.com/deve").unwrap();
        let transport =
            RecordingWebDavTransport::with_response(StatusCode::OK, b"manifest-json".to_vec());

        let outcome = download_webdav_pack_with_transport(
            &locator,
            "Bearer token",
            "deve/branches/writer-1/branch.manifest.enc",
            1024,
            &transport,
        )
        .unwrap();

        assert_eq!(outcome.artifact_bytes, b"manifest-json");
        assert!(outcome.provider_metadata_is_diagnostic_only);
        let calls = transport.get_calls.lock().expect("get calls");
        assert_eq!(
            calls.as_slice(),
            &[(
                "https://dav.example.com/deve/branches/writer-1/branch.manifest.enc".to_string(),
                1024
            )]
        );
    }

    #[test]
    fn webdav_download_rejects_non_success_status() {
        let locator = BackupLocator::parse("webdav+https://dav.example.com/deve").unwrap();
        let transport = RecordingWebDavTransport::with_response(StatusCode::FORBIDDEN, Vec::new());

        let err = download_webdav_pack_with_transport(
            &locator,
            "Bearer token",
            "deve/branches/writer-1/packs/000001.pack.enc",
            1024,
            &transport,
        )
        .expect_err("non-2xx status must fail before restore admission");

        assert!(err.to_string().contains("403"));
    }

    #[test]
    fn webdav_download_rejects_oversized_body() {
        let locator = BackupLocator::parse("webdav+https://dav.example.com/deve").unwrap();
        let transport =
            RecordingWebDavTransport::with_response(StatusCode::OK, b"too-large".to_vec());

        let err = download_webdav_pack_with_transport(
            &locator,
            "Bearer token",
            "deve/branches/writer-1/packs/000001.pack.enc",
            3,
            &transport,
        )
        .expect_err("oversized body must fail closed");

        assert!(err.to_string().contains("exceeded max download bytes"));
    }
}
