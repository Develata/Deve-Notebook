//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::collect::MarkdownProjectionFileRef;
use deve_core::remote_projection::{
    RemoteProjectionAuthorityEffects, RemoteProjectionProvider, RemoteProjectionProviderAdapter,
    RemoteProjectionProviderError, RemoteProjectionPullOutcome, RemoteProjectionPullRequest,
    RemoteProjectionPushOutcome, RemoteProjectionPushRequest,
};
use reqwest::{Method, StatusCode, Url, blocking::Client};
use std::collections::BTreeSet;
use std::fs;
use std::time::Duration;

pub(crate) struct WebDavProjectionProvider<T = ReqwestWebDavTransport> {
    #[cfg(test)]
    pub(super) transport: T,
    #[cfg(not(test))]
    transport: T,
}

impl Default for WebDavProjectionProvider<ReqwestWebDavTransport> {
    fn default() -> Self {
        Self::new().expect("reqwest WebDAV transport with static timeout config")
    }
}

impl WebDavProjectionProvider<ReqwestWebDavTransport> {
    pub(crate) fn new() -> Result<Self, RemoteProjectionProviderError> {
        Ok(Self {
            transport: ReqwestWebDavTransport::new()?,
        })
    }
}

#[cfg(test)]
impl<T> WebDavProjectionProvider<T> {
    pub(super) fn new_for_test(transport: T) -> Self {
        Self { transport }
    }
}

pub(crate) trait WebDavProjectionPushAdapter {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        locator: &str,
        files: &[MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError>;
}

impl<T: WebDavTransport> WebDavProjectionPushAdapter for WebDavProjectionProvider<T> {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        locator: &str,
        files: &[MarkdownProjectionFileRef],
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        if provider != RemoteProjectionProvider::WebDav {
            return Err(RemoteProjectionProviderError::ProviderMismatch);
        }
        let request = RemoteProjectionPushRequest::new(provider, locator, Vec::new())?;
        let base = webdav_locator_to_https_url(request.locator())?;
        let mut ensured_collections = BTreeSet::new();
        ensure_collection(&self.transport, &base, &mut ensured_collections)?;
        for file in files {
            let content = fs::read(file.fs_path()).map_err(|err| {
                RemoteProjectionProviderError::ProviderIo(format!(
                    "failed to read projection file {}: {err}",
                    file.fs_path().display()
                ))
            })?;
            push_payload(
                &self.transport,
                &base,
                &mut ensured_collections,
                file.path(),
                content,
            )?;
        }
        Ok(RemoteProjectionPushOutcome {
            uploaded_files: files.len(),
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            provider_metadata_is_diagnostic_only: true,
        })
    }
}

impl<T: WebDavTransport> RemoteProjectionProviderAdapter for WebDavProjectionProvider<T> {
    fn provider(&self) -> RemoteProjectionProvider {
        RemoteProjectionProvider::WebDav
    }

    fn push(
        &mut self,
        request: RemoteProjectionPushRequest,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        if request.provider() != RemoteProjectionProvider::WebDav {
            return Err(RemoteProjectionProviderError::ProviderMismatch);
        }
        let base = webdav_locator_to_https_url(request.locator())?;
        let mut ensured_collections = BTreeSet::new();
        ensure_collection(&self.transport, &base, &mut ensured_collections)?;
        for file in request.files() {
            push_payload(
                &self.transport,
                &base,
                &mut ensured_collections,
                file.path(),
                file.content().to_vec(),
            )?;
        }
        Ok(RemoteProjectionPushOutcome {
            uploaded_files: request.files().len(),
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            provider_metadata_is_diagnostic_only: true,
        })
    }

    fn pull(
        &self,
        _request: RemoteProjectionPullRequest,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        Err(RemoteProjectionProviderError::ProviderIo(
            "WebDAV pull is not wired in this work item".into(),
        ))
    }
}

#[cfg_attr(not(test), allow(private_bounds))]
pub(super) trait WebDavTransport {
    fn mkcol(&self, url: &Url) -> Result<StatusCode, RemoteProjectionProviderError>;
    fn put(&self, url: &Url, body: Vec<u8>) -> Result<StatusCode, RemoteProjectionProviderError>;
}

pub(crate) struct ReqwestWebDavTransport {
    client: Client,
    mkcol: Method,
}

impl ReqwestWebDavTransport {
    fn new() -> Result<Self, RemoteProjectionProviderError> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(provider_io_error)?;
        Ok(Self {
            client,
            mkcol: Method::from_bytes(b"MKCOL").expect("valid WebDAV MKCOL method"),
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
}

fn push_payload<T: WebDavTransport>(
    transport: &T,
    base: &Url,
    ensured: &mut BTreeSet<String>,
    path: &str,
    content: Vec<u8>,
) -> Result<(), RemoteProjectionProviderError> {
    ensure_parent_collections(transport, base, path, ensured)?;
    let target = webdav_file_url(base, path)?;
    let status = transport.put(&target, content)?;
    if !status.is_success() {
        return Err(RemoteProjectionProviderError::ProviderIo(format!(
            "WebDAV PUT {path} failed with status {}",
            status.as_u16()
        )));
    }
    Ok(())
}

fn ensure_parent_collections<T: WebDavTransport>(
    transport: &T,
    base: &Url,
    file_path: &str,
    ensured: &mut BTreeSet<String>,
) -> Result<(), RemoteProjectionProviderError> {
    let segments = file_path.split('/').collect::<Vec<_>>();
    for end in 1..segments.len() {
        let url = webdav_collection_url(base, &segments[..end])?;
        ensure_collection(transport, &url, ensured)?;
    }
    Ok(())
}

fn ensure_collection<T: WebDavTransport>(
    transport: &T,
    url: &Url,
    ensured: &mut BTreeSet<String>,
) -> Result<(), RemoteProjectionProviderError> {
    if !ensured.insert(url.as_str().to_string()) {
        return Ok(());
    }
    let status = transport.mkcol(url)?;
    if status.is_success() || status == StatusCode::METHOD_NOT_ALLOWED {
        return Ok(());
    }
    Err(RemoteProjectionProviderError::ProviderIo(format!(
        "WebDAV MKCOL {} failed with status {}",
        url,
        status.as_u16()
    )))
}

fn webdav_locator_to_https_url(locator: &str) -> Result<Url, RemoteProjectionProviderError> {
    let https = locator.strip_prefix("webdav+").ok_or_else(|| {
        RemoteProjectionProviderError::ProviderIo("WebDAV locator scheme is invalid".into())
    })?;
    Url::parse(https)
        .map_err(|err| RemoteProjectionProviderError::ProviderIo(format!("invalid URL: {err}")))
}

fn webdav_collection_url(
    base: &Url,
    path_segments: &[&str],
) -> Result<Url, RemoteProjectionProviderError> {
    let mut url = base.clone();
    {
        let mut segments = url.path_segments_mut().map_err(|_| {
            RemoteProjectionProviderError::ProviderIo(
                "WebDAV locator cannot be a base URL for path segments".into(),
            )
        })?;
        for segment in path_segments {
            segments.push(segment);
        }
    }
    Ok(url)
}

fn webdav_file_url(base: &Url, file_path: &str) -> Result<Url, RemoteProjectionProviderError> {
    let segments = file_path.split('/').collect::<Vec<_>>();
    webdav_collection_url(base, &segments)
}

fn provider_io_error(err: reqwest::Error) -> RemoteProjectionProviderError {
    RemoteProjectionProviderError::ProviderIo(err.to_string())
}
