//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract

use super::provider::WebDavProjectionProvider;
use super::transport::WebDavTransport;
use super::url::{webdav_collection_url, webdav_file_url, webdav_locator_to_https_url};
use crate::remote_projection_transport::ProjectionPushSource;
use deve_core::remote_projection::{
    RemoteProjectionAuthorityEffects, RemoteProjectionProvider, RemoteProjectionProviderError,
    RemoteProjectionPushOutcome, RemoteProjectionPushRequest,
};
use reqwest::{StatusCode, Url};
use std::collections::BTreeSet;

pub(crate) trait WebDavProjectionPushAdapter {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        locator: &str,
        source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError>;
}

impl<T: WebDavTransport> WebDavProjectionPushAdapter for WebDavProjectionProvider<T> {
    fn push_projection_files(
        &mut self,
        provider: RemoteProjectionProvider,
        locator: &str,
        source: &dyn ProjectionPushSource,
    ) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
        if provider != RemoteProjectionProvider::WebDav {
            return Err(RemoteProjectionProviderError::ProviderMismatch);
        }
        let request = RemoteProjectionPushRequest::new(provider, locator, Vec::new())?;
        let base = webdav_locator_to_https_url(request.locator())?;
        let mut ensured_collections = BTreeSet::new();
        ensure_collection(&self.transport, &base, &mut ensured_collections)?;
        source.visit(&mut |path, content| {
            push_payload(
                &self.transport,
                &base,
                &mut ensured_collections,
                path,
                content,
            )
        })?;
        Ok(RemoteProjectionPushOutcome {
            uploaded_files: source.file_count(),
            effects: RemoteProjectionAuthorityEffects::projection_transport(),
            provider_metadata_is_diagnostic_only: true,
        })
    }
}

#[cfg(test)]
pub(super) fn push_request<T: WebDavTransport>(
    transport: &T,
    request: RemoteProjectionPushRequest,
) -> Result<RemoteProjectionPushOutcome, RemoteProjectionProviderError> {
    if request.provider() != RemoteProjectionProvider::WebDav {
        return Err(RemoteProjectionProviderError::ProviderMismatch);
    }
    let base = webdav_locator_to_https_url(request.locator())?;
    let mut ensured_collections = BTreeSet::new();
    ensure_collection(transport, &base, &mut ensured_collections)?;
    for file in request.files() {
        push_payload(
            transport,
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
