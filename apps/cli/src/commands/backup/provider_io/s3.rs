//! plan_ref:
//!   - 06_backup#backup-provider-dispatch-contract

mod signing;
mod transport;
mod url;

use super::BackupPackUploadOutcome;
use super::credentials::{S3BackupCredentials, s3_credentials};
use anyhow::bail;
use chrono::Utc;
use deve_core::backup::{BackupLocator, BackupSecretRef};
use signing::signed_put_request;
use transport::{ReqwestS3BackupTransport, S3BackupTransport};
use url::s3_pack_url;

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

fn upload_s3_pack_with_transport<T: S3BackupTransport>(
    locator: &BackupLocator,
    credentials: &S3BackupCredentials,
    object_path: &str,
    artifact_bytes: &[u8],
    transport: &T,
) -> anyhow::Result<BackupPackUploadOutcome> {
    let target = s3_pack_url(locator, &credentials.region, object_path)?;
    let request = signed_put_request(target, artifact_bytes.to_vec(), credentials, Utc::now())?;
    let status = transport.put(request)?;
    if !status.is_success() {
        bail!(
            "Backup S3 PUT {object_path} failed with status {}",
            status.as_u16()
        );
    }
    Ok(BackupPackUploadOutcome {
        uploaded_bytes: artifact_bytes.len(),
        provider_metadata_is_diagnostic_only: true,
    })
}

#[cfg(test)]
mod tests {
    use super::signing::S3SignedBackupPutRequest;
    use super::*;
    use crate::commands::backup::provider_io::BACKUP_PACK_CONTENT_TYPE;
    use reqwest::StatusCode;
    use std::sync::Mutex;

    struct RecordingS3Transport {
        calls: Mutex<Vec<S3SignedBackupPutRequest>>,
        status: StatusCode,
    }

    impl RecordingS3Transport {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                status: StatusCode::OK,
            }
        }

        fn failing(status: StatusCode) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                status,
            }
        }
    }

    impl S3BackupTransport for RecordingS3Transport {
        fn put(&self, request: S3SignedBackupPutRequest) -> anyhow::Result<StatusCode> {
            self.calls.lock().expect("calls").push(request);
            Ok(self.status)
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
    fn s3_upload_puts_backup_pack_bytes_with_backup_content_type() {
        let locator = BackupLocator::parse("s3://bucket-name/deve").unwrap();
        let transport = RecordingS3Transport::new();
        let outcome = upload_s3_pack_with_transport(
            &locator,
            &credentials(),
            "deve/branches/writer-1/packs/000001.pack.enc",
            b"encrypted-json",
            &transport,
        )
        .unwrap();

        assert_eq!(outcome.uploaded_bytes, "encrypted-json".len());
        assert!(outcome.provider_metadata_is_diagnostic_only);
        let calls = transport.calls.lock().expect("calls");
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
