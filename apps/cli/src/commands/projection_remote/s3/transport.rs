//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::signing::S3SignedPutRequest;
use deve_core::remote_projection::RemoteProjectionProviderError;
use reqwest::{
    StatusCode,
    blocking::Client,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use std::time::Duration;

pub(super) trait S3Transport {
    fn put(&self, request: S3SignedPutRequest)
    -> Result<StatusCode, RemoteProjectionProviderError>;
}

pub(crate) struct ReqwestS3Transport {
    client: Client,
}

impl ReqwestS3Transport {
    pub(super) fn new() -> Result<Self, RemoteProjectionProviderError> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(provider_io_error)?;
        Ok(Self { client })
    }
}

impl S3Transport for ReqwestS3Transport {
    fn put(
        &self,
        request: S3SignedPutRequest,
    ) -> Result<StatusCode, RemoteProjectionProviderError> {
        self.client
            .put(request.url)
            .headers(header_map(request.headers)?)
            .body(request.body)
            .send()
            .map(|response| response.status())
            .map_err(provider_io_error)
    }
}

fn header_map(headers: Vec<(String, String)>) -> Result<HeaderMap, RemoteProjectionProviderError> {
    let mut map = HeaderMap::new();
    for (name, value) in headers {
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|err| {
            RemoteProjectionProviderError::ProviderIo(format!("invalid S3 header name: {err}"))
        })?;
        let value = HeaderValue::from_str(&value).map_err(|err| {
            RemoteProjectionProviderError::ProviderIo(format!("invalid S3 header value: {err}"))
        })?;
        map.insert(name, value);
    }
    Ok(map)
}

fn provider_io_error(err: reqwest::Error) -> RemoteProjectionProviderError {
    RemoteProjectionProviderError::ProviderIo(err.to_string())
}
