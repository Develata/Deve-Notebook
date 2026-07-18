use super::super::transport::{WebDavHttpResponse, WebDavStreamResponse, WebDavTransport};
use deve_core::remote_projection::RemoteProjectionProviderError;
use reqwest::{StatusCode, Url};
use std::collections::VecDeque;
use std::sync::Mutex;

#[derive(Debug)]
pub(super) struct RecordingTransport {
    pub(super) calls: Mutex<Vec<String>>,
    mkcol_status: StatusCode,
    put_status: StatusCode,
    propfind_responses: Mutex<VecDeque<WebDavHttpResponse>>,
    pub(super) get_responses: Mutex<VecDeque<WebDavHttpResponse>>,
}

impl RecordingTransport {
    pub(super) fn new(mkcol_status: StatusCode, put_status: StatusCode) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            mkcol_status,
            put_status,
            propfind_responses: Mutex::new(VecDeque::new()),
            get_responses: Mutex::new(VecDeque::new()),
        }
    }

    pub(super) fn with_propfind_body(self, body: String) -> Self {
        self.with_propfind_response(StatusCode::MULTI_STATUS, body.into_bytes())
    }

    pub(super) fn with_propfind_response(self, status: StatusCode, body: Vec<u8>) -> Self {
        self.propfind_responses
            .lock()
            .expect("propfind")
            .push_back(WebDavHttpResponse { status, body });
        self
    }

    pub(super) fn with_get_body(self, body: Vec<u8>) -> Self {
        self.with_get_response(StatusCode::OK, body)
    }

    pub(super) fn with_get_response(self, status: StatusCode, body: Vec<u8>) -> Self {
        self.get_responses
            .lock()
            .expect("get")
            .push_back(WebDavHttpResponse { status, body });
        self
    }
}

impl WebDavTransport for RecordingTransport {
    fn mkcol(&self, url: &Url) -> Result<StatusCode, RemoteProjectionProviderError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("MKCOL {url}"));
        Ok(self.mkcol_status)
    }

    fn put(&self, url: &Url, body: Vec<u8>) -> Result<StatusCode, RemoteProjectionProviderError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("PUT {url} {}", String::from_utf8_lossy(&body)));
        Ok(self.put_status)
    }

    fn propfind(
        &self,
        url: &Url,
        depth: &str,
        max_body_bytes: usize,
    ) -> Result<WebDavHttpResponse, RemoteProjectionProviderError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("PROPFIND {url} depth={depth}"));
        self.propfind_responses
            .lock()
            .expect("propfind")
            .pop_front()
            .ok_or_else(|| RemoteProjectionProviderError::ProviderIo("missing propfind".into()))
            .and_then(|response| limited_response(response, max_body_bytes))
    }

    fn get(&self, url: &Url) -> Result<WebDavStreamResponse, RemoteProjectionProviderError> {
        self.calls.lock().expect("calls").push(format!("GET {url}"));
        self.get_responses
            .lock()
            .expect("get")
            .pop_front()
            .ok_or_else(|| RemoteProjectionProviderError::ProviderIo("missing get".into()))
            .map(|response| WebDavStreamResponse {
                status: response.status,
                body: Box::new(std::io::Cursor::new(response.body)),
            })
    }
}

fn limited_response(
    response: WebDavHttpResponse,
    max_body_bytes: usize,
) -> Result<WebDavHttpResponse, RemoteProjectionProviderError> {
    if response.body.len() > max_body_bytes {
        Err(RemoteProjectionProviderError::ProviderIo(format!(
            "WebDAV response body exceeds {max_body_bytes} bytes"
        )))
    } else {
        Ok(response)
    }
}

pub(super) fn propfind_body(entries: &[(&str, bool)]) -> String {
    let mut body = String::from(r#"<?xml version="1.0"?><d:multistatus xmlns:d="DAV:">"#);
    for (href, is_collection) in entries {
        body.push_str("<d:response><d:href>");
        body.push_str(href);
        body.push_str("</d:href><d:propstat><d:prop><d:resourcetype>");
        if *is_collection {
            body.push_str("<d:collection/>");
        }
        body.push_str("</d:resourcetype></d:prop></d:propstat></d:response>");
    }
    body.push_str("</d:multistatus>");
    body
}
