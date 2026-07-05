//! plan_ref:
//!   - 06_backup#backup-provider-dispatch-contract

use super::signing::S3SignedBackupPutRequest;
use anyhow::Context;
use reqwest::{
    StatusCode,
    blocking::Client,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use std::time::Duration;

pub(super) trait S3BackupTransport {
    fn put(&self, request: S3SignedBackupPutRequest) -> anyhow::Result<StatusCode>;
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

impl S3BackupTransport for ReqwestS3BackupTransport {
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
