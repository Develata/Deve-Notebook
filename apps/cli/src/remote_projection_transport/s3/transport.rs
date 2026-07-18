//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract

use super::signing::{S3SignedGetRequest, S3SignedPutRequest};
use deve_core::remote_projection::RemoteProjectionProviderError;
use reqwest::{
    StatusCode,
    blocking::Client,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use std::io::Read;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct S3HttpResponse {
    pub(super) status: StatusCode,
    pub(super) body: Vec<u8>,
}

pub(super) struct S3StreamResponse {
    pub(super) status: StatusCode,
    pub(super) body: Box<dyn Read + Send>,
}

pub(super) trait S3Transport {
    fn put(&self, request: S3SignedPutRequest)
    -> Result<StatusCode, RemoteProjectionProviderError>;

    fn get(
        &self,
        request: S3SignedGetRequest,
    ) -> Result<S3HttpResponse, RemoteProjectionProviderError>;

    fn get_stream(
        &self,
        request: S3SignedGetRequest,
    ) -> Result<S3StreamResponse, RemoteProjectionProviderError>;
}

pub(crate) struct ReqwestS3Transport {
    client: Client,
}

impl ReqwestS3Transport {
    pub(super) fn new() -> Result<Self, RemoteProjectionProviderError> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .redirect(reqwest::redirect::Policy::none())
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

    fn get(
        &self,
        request: S3SignedGetRequest,
    ) -> Result<S3HttpResponse, RemoteProjectionProviderError> {
        let response = self
            .client
            .get(request.url)
            .headers(header_map(request.headers)?)
            .send()
            .map_err(provider_io_error)?;
        let status = response.status();
        let body = limited_body(response, request.max_body_bytes)?;
        Ok(S3HttpResponse { status, body })
    }

    fn get_stream(
        &self,
        request: S3SignedGetRequest,
    ) -> Result<S3StreamResponse, RemoteProjectionProviderError> {
        let response = self
            .client
            .get(request.url)
            .headers(header_map(request.headers)?)
            .send()
            .map_err(provider_io_error)?;
        Ok(S3StreamResponse {
            status: response.status(),
            body: Box::new(response),
        })
    }
}

fn limited_body(
    response: reqwest::blocking::Response,
    max_body_bytes: usize,
) -> Result<Vec<u8>, RemoteProjectionProviderError> {
    let limit = max_body_bytes.checked_add(1).ok_or_else(|| {
        RemoteProjectionProviderError::ProviderIo("S3 response body byte limit overflow".into())
    })?;
    let mut body = Vec::new();
    response
        .take(limit as u64)
        .read_to_end(&mut body)
        .map_err(|err| RemoteProjectionProviderError::ProviderIo(err.to_string()))?;
    if body.len() > max_body_bytes {
        Err(RemoteProjectionProviderError::ProviderIo(format!(
            "S3 response body exceeds {max_body_bytes} bytes"
        )))
    } else {
        Ok(body)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote_projection_transport::redirect_test_support::assert_redirect_not_followed;

    #[test]
    fn s3_transport_does_not_follow_credential_bearing_redirect() {
        assert_redirect_not_followed(|url| {
            let transport = ReqwestS3Transport::new()?;
            transport
                .get(S3SignedGetRequest {
                    url,
                    headers: vec![("authorization".into(), "signed-secret".into())],
                    max_body_bytes: 1024,
                })
                .map(|response| response.status)
        });
    }
}
