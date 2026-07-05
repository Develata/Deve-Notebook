//! plan_ref:
//!   - 06_backup#backup-provider-dispatch-contract

use super::super::BACKUP_PACK_CONTENT_TYPE;
use anyhow::{Context, bail};
use reqwest::{
    Method, StatusCode, Url,
    blocking::Client,
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue},
};
use std::io::Read;
use std::time::Duration;

#[allow(dead_code)]
pub(super) struct WebDavBackupHttpResponse {
    pub(super) status: StatusCode,
    pub(super) body: Vec<u8>,
}

pub(super) trait WebDavBackupUploadTransport {
    fn mkcol(&self, url: &Url, authorization: &str) -> anyhow::Result<StatusCode>;
    fn put(&self, url: &Url, authorization: &str, body: Vec<u8>) -> anyhow::Result<StatusCode>;
}

pub(super) trait WebDavBackupDownloadTransport {
    #[allow(dead_code)]
    fn get(
        &self,
        url: &Url,
        authorization: &str,
        max_body_bytes: usize,
    ) -> anyhow::Result<WebDavBackupHttpResponse>;
}

pub(super) struct ReqwestWebDavBackupTransport {
    client: Client,
    mkcol: Method,
}

impl ReqwestWebDavBackupTransport {
    pub(super) fn new() -> anyhow::Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .context("failed to build Backup WebDAV client")?;
        Ok(Self {
            client,
            mkcol: Method::from_bytes(b"MKCOL").expect("valid WebDAV MKCOL method"),
        })
    }
}

impl WebDavBackupUploadTransport for ReqwestWebDavBackupTransport {
    fn mkcol(&self, url: &Url, authorization: &str) -> anyhow::Result<StatusCode> {
        let authorization = authorization_header(authorization)?;
        Ok(self
            .client
            .request(self.mkcol.clone(), url.clone())
            .header(AUTHORIZATION, authorization)
            .send()
            .context("Backup WebDAV MKCOL failed")?
            .status())
    }

    fn put(&self, url: &Url, authorization: &str, body: Vec<u8>) -> anyhow::Result<StatusCode> {
        let authorization = authorization_header(authorization)?;
        Ok(self
            .client
            .put(url.clone())
            .header(AUTHORIZATION, authorization)
            .header(CONTENT_TYPE, BACKUP_PACK_CONTENT_TYPE)
            .body(body)
            .send()
            .context("Backup WebDAV PUT failed")?
            .status())
    }
}

impl WebDavBackupDownloadTransport for ReqwestWebDavBackupTransport {
    fn get(
        &self,
        url: &Url,
        authorization: &str,
        max_body_bytes: usize,
    ) -> anyhow::Result<WebDavBackupHttpResponse> {
        let authorization = authorization_header(authorization)?;
        let mut response = self
            .client
            .get(url.clone())
            .header(AUTHORIZATION, authorization)
            .send()
            .context("Backup WebDAV GET failed")?;
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
                .context("Backup WebDAV GET response read failed")?;
            if body.len() > max_body_bytes {
                bail!("Backup WebDAV GET response exceeded max download bytes");
            }
        }
        Ok(WebDavBackupHttpResponse { status, body })
    }
}

fn authorization_header(value: &str) -> anyhow::Result<HeaderValue> {
    HeaderValue::from_str(value).context("backup WebDAV authorization header is invalid")
}
