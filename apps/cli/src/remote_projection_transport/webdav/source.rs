//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract

use super::provider::WebDavProjectionProvider;
use super::transport::WebDavTransport;
use super::url::{relative_path_from_href, webdav_file_url, webdav_locator_to_https_url};
use crate::remote_projection_transport::path_set::{
    DiscoveredRemotePaths, NormalizedDiscoveryPath, RemotePathBudget,
};
use crate::remote_projection_transport::push_source::{
    is_markdown_path, is_reserved_projection_path,
};
use crate::remote_projection_transport::xml::{decoded_xml_ref, decoded_xml_text};
use crate::remote_projection_transport::{
    NormalizedRemotePath, RemoteSourceAcquisition, RemoteSourceSink, SourceAcquisitionError,
    SourceAcquisitionOutcome, SourceAcquisitionRequest,
};
use deve_core::remote_projection::{RemoteProjectionProvider, RemoteProjectionProviderError};
use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::{StatusCode, Url};
use std::collections::{BTreeSet, VecDeque};

pub(super) const MAX_SOURCE_FILE_BYTES: usize = 4 * 1024 * 1024;
pub(super) const MAX_SOURCE_TOTAL_BYTES: usize = 64 * 1024 * 1024;
pub(super) const MAX_SOURCE_COLLECTIONS: usize = 2_048;
const MAX_PROPFIND_BODY_BYTES: usize = 4 * 1024 * 1024;

impl<T: WebDavTransport> RemoteSourceAcquisition for WebDavProjectionProvider<T> {
    fn provider(&self) -> RemoteProjectionProvider {
        RemoteProjectionProvider::WebDav
    }

    fn acquire<S: RemoteSourceSink>(
        &self,
        request: SourceAcquisitionRequest,
        sink: &mut S,
    ) -> Result<SourceAcquisitionOutcome, SourceAcquisitionError<S::Error>> {
        acquire_source(&self.transport, request, sink)
    }
}

pub(super) fn acquire_source<T: WebDavTransport, S: RemoteSourceSink>(
    transport: &T,
    request: SourceAcquisitionRequest,
    sink: &mut S,
) -> Result<SourceAcquisitionOutcome, SourceAcquisitionError<S::Error>> {
    if request.provider() != RemoteProjectionProvider::WebDav {
        return Err(RemoteProjectionProviderError::ProviderMismatch.into());
    }
    let base = webdav_locator_to_https_url(request.locator())?;
    let paths = discover_remote_markdown_files(transport, &base)?;
    let file_count = paths.len();
    let mut total_bytes = 0usize;
    for path in paths {
        let target = webdav_file_url(&base, path.as_str())?;
        let mut response = transport.get(&target)?;
        if response.status != StatusCode::OK {
            return Err(RemoteProjectionProviderError::ProviderIo(format!(
                "WebDAV GET {} failed with status {}",
                path.as_str(),
                response.status.as_u16()
            ))
            .into());
        }
        let bytes = super::super::body::capture_bounded_body(
            "WebDAV",
            &path,
            response.body.as_mut(),
            super::super::body::BodyCaptureBudget {
                max_file_bytes: MAX_SOURCE_FILE_BYTES,
                remaining_total_bytes: MAX_SOURCE_TOTAL_BYTES.saturating_sub(total_bytes),
                max_total_bytes: MAX_SOURCE_TOTAL_BYTES,
            },
            sink,
        )?;
        total_bytes = total_bytes
            .checked_add(bytes)
            .ok_or_else(|| source_budget_error("total downloaded bytes overflow"))?;
        if total_bytes > MAX_SOURCE_TOTAL_BYTES {
            return Err(source_budget_error(format!(
                "WebDAV source acquisition exceeds total byte budget of {MAX_SOURCE_TOTAL_BYTES}"
            ))
            .into());
        }
    }
    Ok(SourceAcquisitionOutcome {
        files: file_count,
        bytes: total_bytes,
    })
}

fn discover_remote_markdown_files<T: WebDavTransport>(
    transport: &T,
    base: &Url,
) -> Result<Vec<NormalizedRemotePath>, RemoteProjectionProviderError> {
    let mut collections = VecDeque::from([(base.clone(), String::new())]);
    let mut scheduled_collection_keys = BTreeSet::from([String::new()]);
    let mut files = DiscoveredRemotePaths::new("WebDAV source acquisition");
    let mut discovery_path_budget = RemotePathBudget::new("WebDAV source discovery");
    while let Some((collection, requested_collection_path)) = collections.pop_front() {
        let response = transport.propfind(&collection, "1", MAX_PROPFIND_BODY_BYTES)?;
        if response.status != StatusCode::MULTI_STATUS {
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
            if entry.is_collection && path == requested_collection_path {
                // A Depth:1 response normally repeats the requested collection.
                // Only that exact self entry is non-discovery metadata; every
                // other duplicate or out-of-root href remains fail-closed.
                continue;
            }
            if is_reserved_projection_path(&path) {
                return Err(RemoteProjectionProviderError::InternalStatePath);
            }
            let path = NormalizedDiscoveryPath::new(path)?;
            discovery_path_budget.observe(path.as_str())?;
            if entry.is_collection {
                let collection = webdav_file_url(base, path.as_str())?;
                let key = path.as_str().to_lowercase();
                if !scheduled_collection_keys.insert(key) {
                    return Err(RemoteProjectionProviderError::DuplicateProjectionPath);
                }
                if scheduled_collection_keys.len() > MAX_SOURCE_COLLECTIONS {
                    return Err(source_budget_error(format!(
                        "WebDAV source acquisition exceeds collection budget of {MAX_SOURCE_COLLECTIONS}"
                    )));
                }
                collections.push_back((collection, path.as_str().to_owned()));
                continue;
            }
            if !is_markdown_path(path.as_str()) {
                return Err(RemoteProjectionProviderError::InvalidProjectionPath);
            }
            files.insert(NormalizedRemotePath::new(path.as_str())?)?;
        }
    }
    Ok(files.into_sorted_vec())
}

#[derive(Debug, Default, PartialEq, Eq)]
struct WebDavPropfindEntry {
    path: Option<String>,
    is_collection: bool,
    href_seen: bool,
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
    let mut href_text = String::new();
    let mut root_open = false;
    let mut root_seen = false;
    let mut root_closed = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let name = event.name();
                let name = name.as_ref();
                if xml_name_is(name, b"multistatus") {
                    if root_seen || root_open || root_closed {
                        return Err(propfind_protocol_error(
                            "WebDAV PROPFIND must contain exactly one multistatus root",
                        ));
                    }
                    root_seen = true;
                    root_open = true;
                } else if !root_open || root_closed {
                    return Err(propfind_protocol_error(
                        "WebDAV PROPFIND content appears outside multistatus",
                    ));
                } else if in_href {
                    return Err(propfind_protocol_error(
                        "WebDAV PROPFIND href contains nested XML",
                    ));
                } else if xml_name_is(name, b"response") {
                    if current.is_some() {
                        return Err(propfind_protocol_error(
                            "WebDAV PROPFIND response elements must not nest",
                        ));
                    }
                    current = Some(WebDavPropfindEntry::default());
                } else if xml_name_is(name, b"href") {
                    let Some(entry) = current.as_ref() else {
                        return Err(propfind_protocol_error(
                            "WebDAV PROPFIND href appears outside response",
                        ));
                    };
                    if entry.href_seen {
                        return Err(propfind_protocol_error(
                            "WebDAV PROPFIND response repeats href",
                        ));
                    }
                    in_href = true;
                    href_text.clear();
                } else if xml_name_is(name, b"collection")
                    && let Some(entry) = current.as_mut()
                {
                    entry.is_collection = true;
                }
            }
            Ok(Event::Empty(event)) => {
                let name = event.name();
                let name = name.as_ref();
                if !root_open
                    || xml_name_is(name, b"multistatus")
                    || xml_name_is(name, b"response")
                    || xml_name_is(name, b"href")
                {
                    return Err(propfind_protocol_error(
                        "WebDAV PROPFIND contains an incomplete empty element",
                    ));
                }
                if xml_name_is(name, b"collection")
                    && let Some(entry) = current.as_mut()
                {
                    entry.is_collection = true;
                }
            }
            Ok(Event::Text(text)) if in_href => {
                href_text.push_str(&decoded_xml_text("WebDAV href", &text)?);
            }
            Ok(Event::GeneralRef(reference)) if in_href => {
                href_text.push_str(&decoded_xml_ref("WebDAV href", &reference)?);
            }
            Ok(Event::End(event)) => {
                let name = event.name();
                let name = name.as_ref();
                if xml_name_is(name, b"multistatus") {
                    if !root_open || current.is_some() || in_href {
                        return Err(propfind_protocol_error(
                            "WebDAV PROPFIND multistatus closes with incomplete response",
                        ));
                    }
                    root_open = false;
                    root_closed = true;
                } else if !root_open {
                    return Err(propfind_protocol_error(
                        "WebDAV PROPFIND closing tag appears outside multistatus",
                    ));
                } else if xml_name_is(name, b"href") {
                    if !in_href {
                        return Err(propfind_protocol_error(
                            "WebDAV PROPFIND href closing tag is unbalanced",
                        ));
                    }
                    in_href = false;
                    if let Some(entry) = current.as_mut() {
                        entry.path = relative_path_from_href(base, &href_text)?;
                        entry.href_seen = true;
                    } else {
                        return Err(propfind_protocol_error(
                            "WebDAV PROPFIND href closes outside response",
                        ));
                    }
                } else if xml_name_is(name, b"response") {
                    if in_href {
                        return Err(propfind_protocol_error(
                            "WebDAV PROPFIND response closes inside href",
                        ));
                    }
                    let Some(entry) = current.take() else {
                        return Err(propfind_protocol_error(
                            "WebDAV PROPFIND response closing tag is unbalanced",
                        ));
                    };
                    if !entry.href_seen {
                        return Err(propfind_protocol_error(
                            "WebDAV PROPFIND response is missing href",
                        ));
                    }
                    entries.push(entry);
                }
            }
            Ok(Event::Text(text)) if !root_open && !text.iter().all(u8::is_ascii_whitespace) => {
                return Err(propfind_protocol_error(
                    "WebDAV PROPFIND contains text outside multistatus",
                ));
            }
            Ok(Event::Eof) => {
                if !root_seen || !root_closed || root_open || current.is_some() || in_href {
                    return Err(propfind_protocol_error(
                        "WebDAV PROPFIND response is empty or truncated",
                    ));
                }
                break;
            }
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

fn source_budget_error(message: impl Into<String>) -> RemoteProjectionProviderError {
    RemoteProjectionProviderError::ProviderIo(message.into())
}

fn propfind_protocol_error(message: impl Into<String>) -> RemoteProjectionProviderError {
    RemoteProjectionProviderError::ProviderIo(message.into())
}
