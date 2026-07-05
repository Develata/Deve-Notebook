//! plan_ref:
//!   - 06_backup#backup-provider-dispatch-contract

use super::signing::{S3SignedBackupGetRequest, S3SignedBackupPutRequest};
use anyhow::{Context, bail};
use reqwest::{
    StatusCode,
    blocking::Client,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use std::io::Read;
use std::time::Duration;

pub(super) trait S3BackupUploadTransport {
    fn put(&self, request: S3SignedBackupPutRequest) -> anyhow::Result<StatusCode>;
}

pub(super) trait S3BackupDownloadTransport {
    #[allow(dead_code)]
    fn get(
        &self,
        request: S3SignedBackupGetRequest,
        max_body_bytes: usize,
    ) -> anyhow::Result<S3BackupHttpResponse>;
}

#[allow(dead_code)]
pub(super) struct S3BackupHttpResponse {
    pub(super) status: StatusCode,
    pub(super) body: Vec<u8>,
}

pub(super) struct ReqwestS3BackupTransport {
    client: Client,
}

impl ReqwestS3BackupTransport {
    pub(super) fn new() -> anyhow::Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .context("failed to build Backup S3 client")?;
        Ok(Self { client })
    }
}

impl S3BackupUploadTransport for ReqwestS3BackupTransport {
    fn put(&self, request: S3SignedBackupPutRequest) -> anyhow::Result<StatusCode> {
        Ok(self
            .client
            .put(request.url)
            .headers(header_map(request.headers)?)
            .body(request.body)
            .send()
            .context("Backup S3 PUT failed")?
            .status())
    }
}

impl S3BackupDownloadTransport for ReqwestS3BackupTransport {
    fn get(
        &self,
        request: S3SignedBackupGetRequest,
        max_body_bytes: usize,
    ) -> anyhow::Result<S3BackupHttpResponse> {
        let mut response = self
            .client
            .get(request.url)
            .headers(header_map(request.headers)?)
            .send()
            .context("Backup S3 GET failed")?;
        let status = response.status();
        let mut body = Vec::new();
        if status.is_success() {
            response
                .by_ref()
                .take(
                    u64::try_from(max_body_bytes)
                        .unwrap_or(u64::MAX)
                        .saturating_add(1),
                )
                .read_to_end(&mut body)
                .context("Backup S3 GET response read failed")?;
            if body.len() > max_body_bytes {
                bail!("Backup S3 GET response exceeded max download bytes");
            }
        }
        Ok(S3BackupHttpResponse { status, body })
    }
}

fn header_map(headers: Vec<(String, String)>) -> anyhow::Result<HeaderMap> {
    let mut map = HeaderMap::new();
    for (name, value) in headers {
        let name = HeaderName::from_bytes(name.as_bytes())
            .with_context(|| format!("invalid Backup S3 header name: {name}"))?;
        let value = HeaderValue::from_str(&value).context("invalid Backup S3 header value")?;
        map.insert(name, value);
    }
    Ok(map)
}
