//! plan_ref:
//!   - 06_backup#backup-provider-dispatch-contract

use super::super::credentials::{S3BackupCredentials, s3_credentials};
use super::super::{
    BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES, BackupPackUploadOutcome, ensure_upload_readback_budget,
    verified_upload_readback_digest,
};
use super::signing::{signed_get_request, signed_put_request};
use super::transport::{
    ReqwestS3BackupTransport, S3BackupDownloadTransport, S3BackupUploadTransport,
};
use super::url::s3_pack_url;
use anyhow::bail;
use chrono::Utc;
use deve_core::backup::{BackupLocator, BackupSecretRef};

pub(super) fn upload_s3_pack(
    locator: &BackupLocator,
    credential_ref: &BackupSecretRef,
    object_path: &str,
    artifact_bytes: &[u8],
) -> anyhow::Result<BackupPackUploadOutcome> {
    let credentials = s3_credentials(credential_ref)?;
    let transport = ReqwestS3BackupTransport::new()?;
    upload_s3_pack_with_transport(
        locator,
        &credentials,
        object_path,
        artifact_bytes,
        &transport,
    )
}

fn upload_s3_pack_with_transport<T: S3BackupUploadTransport + S3BackupDownloadTransport>(
    locator: &BackupLocator,
    credentials: &S3BackupCredentials,
    object_path: &str,
    artifact_bytes: &[u8],
    transport: &T,
) -> anyhow::Result<BackupPackUploadOutcome> {
    ensure_upload_readback_budget(object_path, artifact_bytes.len())?;
    let target = s3_pack_url(locator, &credentials.region, object_path)?;
    let request = signed_put_request(
        target.clone(),
        artifact_bytes.to_vec(),
        credentials,
        Utc::now(),
    )?;
    let status = transport.put(request)?;
    if !status.is_success() {
        bail!(
            "Backup S3 PUT {object_path} failed with status {}",
            status.as_u16()
        );
    }
    let request = signed_get_request(target, credentials, Utc::now())?;
    let response = transport.get(request, BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES)?;
    if !response.status.is_success() {
        bail!(
            "Backup S3 readback {object_path} failed with status {}",
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

#[cfg(test)]
mod tests {
    use super::super::signing::{S3SignedBackupGetRequest, S3SignedBackupPutRequest};
    use super::super::transport::S3BackupHttpResponse;
    use super::*;
    use crate::commands::backup::provider_io::BACKUP_PACK_CONTENT_TYPE;
    use reqwest::StatusCode;
    use std::sync::Mutex;

    struct RecordingS3Transport {
        put_calls: Mutex<Vec<S3SignedBackupPutRequest>>,
        get_calls: Mutex<Vec<(S3SignedBackupGetRequest, usize)>>,
        put_status: StatusCode,
        get_response: Mutex<Option<S3BackupHttpResponse>>,
    }

    impl RecordingS3Transport {
        fn with_readback(body: Vec<u8>) -> Self {
            Self::with_readback_response(StatusCode::OK, body)
        }

        fn with_readback_response(status: StatusCode, body: Vec<u8>) -> Self {
            Self {
                put_calls: Mutex::new(Vec::new()),
                get_calls: Mutex::new(Vec::new()),
                put_status: StatusCode::OK,
                get_response: Mutex::new(Some(S3BackupHttpResponse { status, body })),
            }
        }

        fn failing(status: StatusCode) -> Self {
            Self {
                put_calls: Mutex::new(Vec::new()),
                get_calls: Mutex::new(Vec::new()),
                put_status: status,
                get_response: Mutex::new(None),
            }
        }
    }

    impl S3BackupUploadTransport for RecordingS3Transport {
        fn put(&self, request: S3SignedBackupPutRequest) -> anyhow::Result<StatusCode> {
            self.put_calls.lock().expect("put calls").push(request);
            Ok(self.put_status)
        }
    }

    impl S3BackupDownloadTransport for RecordingS3Transport {
        fn get(
            &self,
            request: S3SignedBackupGetRequest,
            max_body_bytes: usize,
        ) -> anyhow::Result<S3BackupHttpResponse> {
            self.get_calls
                .lock()
                .expect("get calls")
                .push((request, max_body_bytes));
            Ok(self
                .get_response
                .lock()
                .expect("get response")
                .take()
                .expect("get response configured"))
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
    fn s3_upload_remote_verifies_readback_bytes() {
        let locator = BackupLocator::parse("s3://bucket-name/deve").unwrap();
        let transport = RecordingS3Transport::with_readback(b"encrypted-json".to_vec());
        let outcome = upload_s3_pack_with_transport(
            &locator,
            &credentials(),
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
        let calls = transport.put_calls.lock().expect("put calls");
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].url.as_str(),
            "https://bucket-name.s3.us-east-1.amazonaws.com/deve/branches/writer-1/packs/000001.pack.enc"
        );
        assert_eq!(calls[0].body, b"encrypted-json");
        assert!(
            calls[0].headers.iter().any(|(name, value)| {
                name == "content-type" && value == BACKUP_PACK_CONTENT_TYPE
            })
        );
        let get_calls = transport.get_calls.lock().expect("get calls");
        assert_eq!(get_calls.len(), 1);
        assert_eq!(
            get_calls[0].0.url.as_str(),
            "https://bucket-name.s3.us-east-1.amazonaws.com/deve/branches/writer-1/packs/000001.pack.enc"
        );
        assert_eq!(get_calls[0].1, BACKUP_ARTIFACT_MAX_DOWNLOAD_BYTES);
        assert!(
            !get_calls[0]
                .0
                .headers
                .iter()
                .any(|(name, _)| name == "content-type")
        );
    }

    #[test]
    fn s3_upload_rejects_readback_mismatch() {
        let locator = BackupLocator::parse("s3://bucket-name/deve").unwrap();
        let transport = RecordingS3Transport::with_readback(b"tampered-json".to_vec());

        let err = upload_s3_pack_with_transport(
            &locator,
            &credentials(),
            "deve/branches/writer-1/packs/000001.pack.enc",
            b"encrypted-json",
            &transport,
        )
        .expect_err("remote readback mismatch must fail closed");

        assert!(err.to_string().contains("readback"));
    }

    #[test]
    fn s3_upload_rejects_readback_non_success_status() {
        let locator = BackupLocator::parse("s3://bucket-name/deve").unwrap();
        let transport =
            RecordingS3Transport::with_readback_response(StatusCode::NOT_FOUND, Vec::new());

        let err = upload_s3_pack_with_transport(
            &locator,
            &credentials(),
            "deve/branches/writer-1/packs/000001.pack.enc",
            b"encrypted-json",
            &transport,
        )
        .expect_err("remote readback non-success status must fail closed");

        assert!(err.to_string().contains("readback"));
        assert!(err.to_string().contains("404"));
    }

    #[test]
    fn s3_upload_rejects_non_success_status() {
        let locator = BackupLocator::parse("s3://bucket-name/deve").unwrap();
        let transport = RecordingS3Transport::failing(StatusCode::FORBIDDEN);

        let err = upload_s3_pack_with_transport(
            &locator,
            &credentials(),
            "deve/branches/writer-1/packs/000001.pack.enc",
            b"encrypted-json",
            &transport,
        )
        .expect_err("non-2xx status must fail before Uploaded state");

        assert!(err.to_string().contains("403"));
    }
}
