//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::super::credentials::S3Credentials;
use super::super::signing::{S3SignedGetRequest, S3SignedPutRequest};
use super::super::transport::{S3HttpResponse, S3Transport};
use chrono::{TimeZone, Utc};
use deve_core::remote_projection::RemoteProjectionProviderError;
use reqwest::StatusCode;
use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard};

static ENV_LOCK: Mutex<()> = Mutex::new(());

pub(super) fn test_credentials() -> S3Credentials {
    S3Credentials::for_test()
}

#[derive(Debug)]
pub(super) struct RecordingS3Transport {
    pub(super) put_calls: Mutex<Vec<S3SignedPutRequest>>,
    pub(super) get_calls: Mutex<Vec<S3SignedGetRequest>>,
    put_status: StatusCode,
    get_responses: Mutex<VecDeque<S3HttpResponse>>,
}

impl RecordingS3Transport {
    pub(super) fn new(put_status: StatusCode) -> Self {
        Self {
            put_calls: Mutex::new(Vec::new()),
            get_calls: Mutex::new(Vec::new()),
            put_status,
            get_responses: Mutex::new(VecDeque::new()),
        }
    }

    pub(super) fn with_get_body(self, body: Vec<u8>) -> Self {
        self.with_get_response(StatusCode::OK, body)
    }

    pub(super) fn with_get_response(self, status: StatusCode, body: Vec<u8>) -> Self {
        self.get_responses
            .lock()
            .expect("get")
            .push_back(S3HttpResponse { status, body });
        self
    }
}

impl S3Transport for RecordingS3Transport {
    fn put(
        &self,
        request: S3SignedPutRequest,
    ) -> Result<StatusCode, RemoteProjectionProviderError> {
        self.put_calls.lock().expect("calls").push(request);
        Ok(self.put_status)
    }

    fn get(
        &self,
        request: S3SignedGetRequest,
    ) -> Result<S3HttpResponse, RemoteProjectionProviderError> {
        let max_body_bytes = request.max_body_bytes;
        self.get_calls.lock().expect("calls").push(request);
        self.get_responses
            .lock()
            .expect("get")
            .pop_front()
            .ok_or_else(|| RemoteProjectionProviderError::ProviderIo("missing get".into()))
            .and_then(|response| limited_response(response, max_body_bytes))
    }
}

fn limited_response(
    response: S3HttpResponse,
    max_body_bytes: usize,
) -> Result<S3HttpResponse, RemoteProjectionProviderError> {
    if response.body.len() > max_body_bytes {
        Err(RemoteProjectionProviderError::ProviderIo(format!(
            "S3 response body exceeds {max_body_bytes} bytes"
        )))
    } else {
        Ok(response)
    }
}

pub(super) fn header(request: &S3SignedPutRequest, name: &str) -> String {
    request
        .headers
        .iter()
        .find_map(|(header_name, value)| (header_name == name).then(|| value.clone()))
        .unwrap_or_else(|| panic!("missing header {name}"))
}

pub(super) fn get_header(request: &S3SignedGetRequest, name: &str) -> String {
    request
        .headers
        .iter()
        .find_map(|(header_name, value)| (header_name == name).then(|| value.clone()))
        .unwrap_or_else(|| panic!("missing header {name}"))
}

pub(super) fn s3_list_body(keys: &[&str], next_continuation_token: Option<&str>) -> Vec<u8> {
    let mut body = String::from("<ListBucketResult>");
    body.push_str("<IsTruncated>");
    body.push_str(if next_continuation_token.is_some() {
        "true"
    } else {
        "false"
    });
    body.push_str("</IsTruncated>");
    for key in keys {
        body.push_str("<Contents><Key>");
        body.push_str(&xml_escape(key));
        body.push_str("</Key></Contents>");
    }
    if let Some(token) = next_continuation_token {
        body.push_str("<NextContinuationToken>");
        body.push_str(&xml_escape(token));
        body.push_str("</NextContinuationToken>");
    }
    body.push_str("</ListBucketResult>");
    body.into_bytes()
}

pub(super) fn s3_truncated_list_body_without_token(keys: &[&str]) -> Vec<u8> {
    let mut body = String::from("<ListBucketResult><IsTruncated>true</IsTruncated>");
    for key in keys {
        body.push_str("<Contents><Key>");
        body.push_str(key);
        body.push_str("</Key></Contents>");
    }
    body.push_str("</ListBucketResult>");
    body.into_bytes()
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub(super) fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 5, 12, 0, 0)
        .single()
        .expect("time")
}

pub(super) struct EnvGuard {
    _lock: MutexGuard<'static, ()>,
    old: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    pub(super) fn set(values: &[(&'static str, Option<&'static str>)]) -> Self {
        let lock = ENV_LOCK.lock().expect("env lock");
        let old = values
            .iter()
            .map(|(key, _)| (*key, std::env::var(key).ok()))
            .collect::<Vec<_>>();
        for (key, value) in values {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
        Self { _lock: lock, old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.old.drain(..) {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}
