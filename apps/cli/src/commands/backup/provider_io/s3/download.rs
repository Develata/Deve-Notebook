//! plan_ref:
//!   - 06_backup#backup-provider-dispatch-contract
//!   - 06_backup#backup-restore-state-machine-contract

use super::super::BackupArtifactDownloadOutcome;
use super::super::credentials::{S3BackupCredentials, s3_credentials};
use super::signing::signed_get_request;
use super::transport::{ReqwestS3BackupTransport, S3BackupDownloadTransport};
use super::url::s3_pack_url;
use anyhow::bail;
use chrono::Utc;
use deve_core::backup::{BackupLocator, BackupSecretRef};

#[allow(dead_code)]
pub(super) fn download_s3_pack(
    locator: &BackupLocator,
    credential_ref: &BackupSecretRef,
    object_path: &str,
    max_bytes: usize,
) -> anyhow::Result<BackupArtifactDownloadOutcome> {
    let credentials = s3_credentials(credential_ref)?;
    let transport = ReqwestS3BackupTransport::new()?;
    download_s3_pack_with_transport(locator, &credentials, object_path, max_bytes, &transport)
}

#[allow(dead_code)]
fn download_s3_pack_with_transport<T: S3BackupDownloadTransport>(
    locator: &BackupLocator,
    credentials: &S3BackupCredentials,
    object_path: &str,
    max_bytes: usize,
    transport: &T,
) -> anyhow::Result<BackupArtifactDownloadOutcome> {
    let target = s3_pack_url(locator, &credentials.region, object_path)?;
    let request = signed_get_request(target, credentials, Utc::now())?;
    let response = transport.get(request, max_bytes)?;
    if !response.status.is_success() {
        bail!(
            "Backup S3 GET {object_path} failed with status {}",
            response.status.as_u16()
        );
    }
    if response.body.len() > max_bytes {
        bail!("Backup S3 GET {object_path} exceeded max download bytes");
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
    use super::super::signing::S3SignedBackupGetRequest;
    use super::super::transport::S3BackupHttpResponse;
    use super::*;
    use reqwest::StatusCode;
    use std::sync::Mutex;

    struct RecordingS3Transport {
        calls: Mutex<Vec<(S3SignedBackupGetRequest, usize)>>,
        response: Mutex<Option<S3BackupHttpResponse>>,
    }

    impl RecordingS3Transport {
        fn with_response(status: StatusCode, body: Vec<u8>) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                response: Mutex::new(Some(S3BackupHttpResponse { status, body })),
            }
        }
    }

    impl S3BackupDownloadTransport for RecordingS3Transport {
        fn get(
            &self,
            request: S3SignedBackupGetRequest,
            max_body_bytes: usize,
        ) -> anyhow::Result<S3BackupHttpResponse> {
            self.calls
                .lock()
                .expect("calls")
                .push((request, max_body_bytes));
            Ok(self
                .response
                .lock()
                .expect("response")
                .take()
                .expect("response configured"))
        }
    }

    fn credentials() -> S3BackupCredentials {
        S3BackupCredentials {
            access_key_id: "AKIDEXAMPLE".into(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".into(),
            session_token: None,
            region: "us-east-1".into(),
        }
    }

    #[test]
    fn s3_download_returns_encrypted_bytes_only() {
        let locator = BackupLocator::parse("s3://bucket-name/deve").unwrap();
        let transport =
            RecordingS3Transport::with_response(StatusCode::OK, b"encrypted-json".to_vec());

        let outcome = download_s3_pack_with_transport(
            &locator,
            &credentials(),
            "deve/branches/writer-1/packs/000001.pack.enc",
            1024,
            &transport,
        )
        .unwrap();

        assert_eq!(outcome.artifact_bytes, b"encrypted-json");
        assert_eq!(outcome.downloaded_bytes, "encrypted-json".len());
        assert!(outcome.provider_metadata_is_diagnostic_only);
        let calls = transport.calls.lock().expect("calls");
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].0.url.as_str(),
            "https://bucket-name.s3.us-east-1.amazonaws.com/deve/branches/writer-1/packs/000001.pack.enc"
        );
        assert_eq!(calls[0].1, 1024);
        assert!(
            !calls[0]
                .0
                .headers
                .iter()
                .any(|(name, _)| name == "content-type")
        );
    }

    #[test]
    fn s3_download_accepts_branch_manifest_artifact_path() {
        let locator = BackupLocator::parse("s3://bucket-name/deve").unwrap();
        let transport =
            RecordingS3Transport::with_response(StatusCode::OK, b"manifest-json".to_vec());

        let outcome = download_s3_pack_with_transport(
            &locator,
            &credentials(),
            "deve/branches/writer-1/branch.manifest.enc",
            1024,
            &transport,
        )
        .unwrap();

        assert_eq!(outcome.artifact_bytes, b"manifest-json");
        assert!(outcome.provider_metadata_is_diagnostic_only);
        let calls = transport.calls.lock().expect("calls");
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].0.url.as_str(),
            "https://bucket-name.s3.us-east-1.amazonaws.com/deve/branches/writer-1/branch.manifest.enc"
        );
        assert_eq!(calls[0].1, 1024);
    }

    #[test]
    fn s3_download_rejects_non_success_status() {
        let locator = BackupLocator::parse("s3://bucket-name/deve").unwrap();
        let transport = RecordingS3Transport::with_response(StatusCode::FORBIDDEN, Vec::new());

        let err = download_s3_pack_with_transport(
            &locator,
            &credentials(),
            "deve/branches/writer-1/packs/000001.pack.enc",
            1024,
            &transport,
        )
        .expect_err("non-2xx status must fail before restore admission");

        assert!(err.to_string().contains("403"));
    }

    #[test]
    fn s3_download_rejects_oversized_body() {
        let locator = BackupLocator::parse("s3://bucket-name/deve").unwrap();
        let transport = RecordingS3Transport::with_response(StatusCode::OK, b"too-large".to_vec());

        let err = download_s3_pack_with_transport(
            &locator,
            &credentials(),
            "deve/branches/writer-1/packs/000001.pack.enc",
            3,
            &transport,
        )
        .expect_err("oversized body must fail closed");

        assert!(err.to_string().contains("exceeded max download bytes"));
    }
}
