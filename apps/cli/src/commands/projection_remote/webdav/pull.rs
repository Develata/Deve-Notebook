//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::provider::WebDavProjectionProvider;
use super::transport::WebDavTransport;
use super::url::{relative_path_from_href, webdav_file_url, webdav_locator_to_https_url};
use crate::commands::projection_remote::collect::{is_markdown_path, is_reserved_projection_path};
use crate::commands::projection_remote::workspace_apply::write_pull_files;
use deve_core::remote_projection::{
    RemoteProjectionAuthorityEffects, RemoteProjectionFile, RemoteProjectionProvider,
    RemoteProjectionProviderError, RemoteProjectionPullOutcome, RemoteProjectionPullRequest,
};
use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::{StatusCode, Url};
use std::collections::{BTreeSet, VecDeque};
use std::path::Path;

pub(super) const MAX_PULL_FILES: usize = 2_048;
pub(super) const MAX_PULL_FILE_BYTES: usize = 4 * 1024 * 1024;
const MAX_PULL_TOTAL_BYTES: usize = 64 * 1024 * 1024;
const MAX_PULL_COLLECTIONS: usize = 2_048;
const MAX_PROPFIND_BODY_BYTES: usize = 4 * 1024 * 1024;

pub(crate) trait WebDavProjectionPullAdapter {
    fn pull_projection_files(
        &self,
        provider: RemoteProjectionProvider,
        locator: &str,
        workspace: &Path,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError>;
}

impl<T: WebDavTransport> WebDavProjectionPullAdapter for WebDavProjectionProvider<T> {
    fn pull_projection_files(
        &self,
        provider: RemoteProjectionProvider,
        locator: &str,
        workspace: &Path,
    ) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
        if provider != RemoteProjectionProvider::WebDav {
            return Err(RemoteProjectionProviderError::ProviderMismatch);
        }
        let request = RemoteProjectionPullRequest::new(provider, locator)?;
        let outcome = pull_request(&self.transport, request)?;
        write_pull_files(workspace, &outcome.files)?;
        Ok(outcome)
    }
}

pub(super) fn pull_request<T: WebDavTransport>(
    transport: &T,
    request: RemoteProjectionPullRequest,
) -> Result<RemoteProjectionPullOutcome, RemoteProjectionProviderError> {
    if request.provider() != RemoteProjectionProvider::WebDav {
        return Err(RemoteProjectionProviderError::ProviderMismatch);
    }
    let base = webdav_locator_to_https_url(request.locator())?;
    let paths = discover_remote_markdown_files(transport, &base)?;
    let mut files = Vec::new();
    let mut total_bytes = 0usize;
    for path in paths {
        let target = webdav_file_url(&base, &path)?;
        let response = transport.get(&target, MAX_PULL_FILE_BYTES)?;
        if !response.status.is_success() {
            return Err(RemoteProjectionProviderError::ProviderIo(format!(
                "WebDAV GET {path} failed with status {}",
                response.status.as_u16()
            )));
        }
        total_bytes = total_bytes
            .checked_add(response.body.len())
            .ok_or_else(|| pull_budget_error("total downloaded bytes overflow"))?;
        if total_bytes > MAX_PULL_TOTAL_BYTES {
            return Err(pull_budget_error(format!(
                "WebDAV pull exceeds total byte budget of {MAX_PULL_TOTAL_BYTES}"
            )));
        }
        files.push(RemoteProjectionFile::new(&path, response.body)?);
    }
    Ok(RemoteProjectionPullOutcome {
        files,
        effects: RemoteProjectionAuthorityEffects::projection_transport(),
        overwrites_projection_workspace: true,
        external_changes_confirmation_required: true,
        provider_metadata_is_diagnostic_only: true,
    })
}

fn discover_remote_markdown_files<T: WebDavTransport>(
    transport: &T,
    base: &Url,
) -> Result<Vec<String>, RemoteProjectionProviderError> {
    let mut collections = VecDeque::from([base.clone()]);
    let mut seen_collections = BTreeSet::new();
    let mut files = BTreeSet::new();
    while let Some(collection) = collections.pop_front() {
        if !seen_collections.insert(collection.as_str().to_string()) {
            continue;
        }
        if seen_collections.len() > MAX_PULL_COLLECTIONS {
            return Err(pull_budget_error(format!(
                "WebDAV pull exceeds collection budget of {MAX_PULL_COLLECTIONS}"
            )));
        }
        let response = transport.propfind(&collection, "1", MAX_PROPFIND_BODY_BYTES)?;
        if !response.status.is_success() && response.status != StatusCode::MULTI_STATUS {
            return Err(RemoteProjectionProviderError::ProviderIo(format!(
                "WebDAV PROPFIND {} failed with status {}",
                collection,
                response.status.as_u16()
            )));
        }
        for entry in parse_propfind_entries(base, &response.body)? {
            let Some(path) = entry.path else {
                continue;
            };
            if is_reserved_projection_path(&path) {
                continue;
            }
            if entry.is_collection {
                collections.push_back(webdav_file_url(base, &path)?);
                continue;
            }
            if is_markdown_path(&path) {
                RemoteProjectionFile::new(&path, Vec::new())?;
                files.insert(path);
                if files.len() > MAX_PULL_FILES {
                    return Err(pull_budget_error(format!(
                        "WebDAV pull exceeds file budget of {MAX_PULL_FILES}"
                    )));
                }
            }
        }
    }
    Ok(files.into_iter().collect())
}

#[derive(Debug, Default, PartialEq, Eq)]
struct WebDavPropfindEntry {
    path: Option<String>,
    is_collection: bool,
}

fn parse_propfind_entries(
    base: &Url,
    body: &[u8],
) -> Result<Vec<WebDavPropfindEntry>, RemoteProjectionProviderError> {
    let xml = std::str::from_utf8(body).map_err(|err| {
        RemoteProjectionProviderError::ProviderIo(format!(
            "WebDAV PROPFIND body is not UTF-8: {err}"
        ))
    })?;
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut entries = Vec::new();
    let mut current: Option<WebDavPropfindEntry> = None;
    let mut in_href = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let name = event.name();
                let name = name.as_ref();
                if xml_name_is(name, b"response") {
                    current = Some(WebDavPropfindEntry::default());
                } else if xml_name_is(name, b"href") {
                    in_href = true;
                } else if xml_name_is(name, b"collection")
                    && let Some(entry) = current.as_mut()
                {
                    entry.is_collection = true;
                }
            }
            Ok(Event::Empty(event)) => {
                let name = event.name();
                let name = name.as_ref();
                if xml_name_is(name, b"collection")
                    && let Some(entry) = current.as_mut()
                {
                    entry.is_collection = true;
                }
            }
            Ok(Event::Text(text)) if in_href => {
                if let Some(entry) = current.as_mut() {
                    let href = String::from_utf8_lossy(text.as_ref());
                    entry.path = relative_path_from_href(base, &href)?;
                }
            }
            Ok(Event::End(event)) => {
                let name = event.name();
                let name = name.as_ref();
                if xml_name_is(name, b"href") {
                    in_href = false;
                } else if xml_name_is(name, b"response")
                    && let Some(entry) = current.take()
                {
                    entries.push(entry);
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                return Err(RemoteProjectionProviderError::ProviderIo(format!(
                    "failed to parse WebDAV PROPFIND response: {err}"
                )));
            }
            _ => {}
        }
    }
    Ok(entries)
}

fn xml_name_is(name: &[u8], local: &[u8]) -> bool {
    name == local
        || name
            .strip_suffix(local)
            .is_some_and(|prefix| prefix.ends_with(b":"))
}

fn pull_budget_error(message: impl Into<String>) -> RemoteProjectionProviderError {
    RemoteProjectionProviderError::ProviderIo(message.into())
}
