//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use deve_core::remote_projection::RemoteProjectionProviderError;
use reqwest::{Method, StatusCode, Url, blocking::Client};
use std::io::Read;
use std::time::Duration;

#[derive(Debug, Clone)]
pub(super) struct WebDavHttpResponse {
    pub(super) status: StatusCode,
    pub(super) body: Vec<u8>,
}

pub(super) trait WebDavTransport {
    fn mkcol(&self, url: &Url) -> Result<StatusCode, RemoteProjectionProviderError>;
    fn put(&self, url: &Url, body: Vec<u8>) -> Result<StatusCode, RemoteProjectionProviderError>;
    fn propfind(
        &self,
        url: &Url,
        depth: &str,
        max_body_bytes: usize,
    ) -> Result<WebDavHttpResponse, RemoteProjectionProviderError>;
    fn get(
        &self,
        url: &Url,
        max_body_bytes: usize,
    ) -> Result<WebDavHttpResponse, RemoteProjectionProviderError>;
}

pub(crate) struct ReqwestWebDavTransport {
    client: Client,
    mkcol: Method,
    propfind: Method,
}

impl ReqwestWebDavTransport {
    pub(super) fn new() -> Result<Self, RemoteProjectionProviderError> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(provider_io_error)?;
        Ok(Self {
            client,
            mkcol: Method::from_bytes(b"MKCOL").expect("valid WebDAV MKCOL method"),
            propfind: Method::from_bytes(b"PROPFIND").expect("valid WebDAV PROPFIND method"),
        })
    }
}

impl WebDavTransport for ReqwestWebDavTransport {
    fn mkcol(&self, url: &Url) -> Result<StatusCode, RemoteProjectionProviderError> {
        self.client
            .request(self.mkcol.clone(), url.clone())
            .send()
            .map(|response| response.status())
            .map_err(provider_io_error)
    }

    fn put(&self, url: &Url, body: Vec<u8>) -> Result<StatusCode, RemoteProjectionProviderError> {
        self.client
            .put(url.clone())
            .header(
                reqwest::header::CONTENT_TYPE,
                "text/markdown; charset=utf-8",
            )
            .body(body)
            .send()
            .map(|response| response.status())
            .map_err(provider_io_error)
    }

    fn propfind(
        &self,
        url: &Url,
        depth: &str,
        max_body_bytes: usize,
    ) -> Result<WebDavHttpResponse, RemoteProjectionProviderError> {
        let response = self
            .client
            .request(self.propfind.clone(), url.clone())
            .header("Depth", depth)
            .header(reqwest::header::CONTENT_TYPE, "application/xml; charset=utf-8")
            .body(r#"<?xml version="1.0"?><d:propfind xmlns:d="DAV:"><d:prop><d:resourcetype/></d:prop></d:propfind>"#)
            .send()
            .map_err(provider_io_error)?;
        http_response(response, max_body_bytes)
    }

    fn get(
        &self,
        url: &Url,
        max_body_bytes: usize,
    ) -> Result<WebDavHttpResponse, RemoteProjectionProviderError> {
        let response = self
            .client
            .get(url.clone())
            .send()
            .map_err(provider_io_error)?;
        http_response(response, max_body_bytes)
    }
}

fn http_response(
    mut response: reqwest::blocking::Response,
    max_body_bytes: usize,
) -> Result<WebDavHttpResponse, RemoteProjectionProviderError> {
    let status = response.status();
    let mut body = Vec::new();
    response
        .by_ref()
        .take(max_body_bytes.saturating_add(1) as u64)
        .read_to_end(&mut body)
        .map_err(|err| RemoteProjectionProviderError::ProviderIo(err.to_string()))?;
    if body.len() > max_body_bytes {
        return Err(RemoteProjectionProviderError::ProviderIo(format!(
            "WebDAV response body exceeds {max_body_bytes} bytes"
        )));
    }
    Ok(WebDavHttpResponse { status, body })
}

fn provider_io_error(err: reqwest::Error) -> RemoteProjectionProviderError {
    RemoteProjectionProviderError::ProviderIo(err.to_string())
}
