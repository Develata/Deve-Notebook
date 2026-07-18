//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract
//!   - 06_backup#projection-backup-remote-layout-contract

use super::credentials::S3Credentials;
use super::signing::signed_get_request;
use super::transport::S3Transport;
use super::url::{
    S3CustomEndpointUrlBinding, s3_list_url_with_binding, s3_locator_prefix_with_binding,
};
use crate::remote_projection_transport::NormalizedRemotePath;
use crate::remote_projection_transport::path_set::DiscoveredRemotePaths;
use crate::remote_projection_transport::push_source::{
    is_markdown_path, is_reserved_projection_path,
};
use crate::remote_projection_transport::xml::{decoded_xml_ref, decoded_xml_text};
use chrono::{DateTime, Utc};
use deve_core::remote_projection::RemoteProjectionProviderError;
use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::StatusCode;
use std::collections::BTreeSet;
const MAX_LIST_BODY_BYTES: usize = 4 * 1024 * 1024;
const MAX_LIST_PAGES: usize = 2_048;

pub(super) fn discover_remote_markdown_files<T: S3Transport>(
    transport: &T,
    credentials: &S3Credentials,
    region: &str,
    custom_url_binding: Option<&S3CustomEndpointUrlBinding>,
    now: fn() -> DateTime<Utc>,
    locator: &str,
) -> Result<Vec<NormalizedRemotePath>, RemoteProjectionProviderError> {
    let prefix = s3_locator_prefix_with_binding(locator, custom_url_binding)?;
    let mut continuation_token = None;
    let mut seen_continuation_tokens = BTreeSet::new();
    let mut pages = 0usize;
    let mut files = DiscoveredRemotePaths::new("S3 source acquisition");
    loop {
        pages += 1;
        if pages > MAX_LIST_PAGES {
            return Err(list_budget_error(format!(
                "S3 source acquisition exceeds list page budget of {MAX_LIST_PAGES}"
            )));
        }
        let list_url = s3_list_url_with_binding(
            locator,
            region,
            continuation_token.as_deref(),
            custom_url_binding,
        )?;
        let response = transport.get(signed_get_request(
            list_url,
            credentials,
            region,
            now(),
            MAX_LIST_BODY_BYTES,
        )?)?;
        if response.status != StatusCode::OK {
            return Err(RemoteProjectionProviderError::ProviderIo(format!(
                "S3 LIST failed with status {}",
                response.status.as_u16()
            )));
        }
        let page = parse_list_objects_v2_page(&response.body, &prefix)?;
        let next_continuation_token = match (page.is_truncated, page.next_continuation_token) {
            (false, None) => None,
            (false, Some(_)) => {
                return Err(list_protocol_error(
                    "S3 ListObjectsV2 response provides a continuation token while IsTruncated=false",
                ));
            }
            (true, None) => {
                return Err(list_protocol_error(
                    "S3 ListObjectsV2 response is truncated without a continuation token",
                ));
            }
            (true, Some(token)) => {
                if !seen_continuation_tokens.insert(token.clone()) {
                    return Err(list_protocol_error(
                        "S3 ListObjectsV2 continuation token repeated",
                    ));
                }
                Some(token)
            }
        };
        for path in page.paths {
            if is_reserved_projection_path(&path) {
                return Err(RemoteProjectionProviderError::InternalStatePath);
            }
            if !is_markdown_path(&path) {
                return Err(RemoteProjectionProviderError::InvalidProjectionPath);
            }
            let path = NormalizedRemotePath::new(path)?;
            files.insert(path)?;
        }
        continuation_token = next_continuation_token;
        if continuation_token.is_none() {
            break;
        }
    }
    Ok(files.into_sorted_vec())
}

#[derive(Debug, Default, PartialEq, Eq)]
struct S3ListPage {
    paths: Vec<String>,
    is_truncated: bool,
    next_continuation_token: Option<String>,
}

fn parse_list_objects_v2_page(
    body: &[u8],
    prefix: &str,
) -> Result<S3ListPage, RemoteProjectionProviderError> {
    let xml = std::str::from_utf8(body).map_err(|err| {
        RemoteProjectionProviderError::ProviderIo(format!(
            "S3 ListObjectsV2 body is not UTF-8: {err}"
        ))
    })?;
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut page = S3ListPage::default();
    let mut in_key = false;
    let mut in_is_truncated = false;
    let mut in_next_token = false;
    let mut key_text = String::new();
    let mut truncated_text = String::new();
    let mut next_token_text = String::new();
    let mut root_open = false;
    let mut root_seen = false;
    let mut root_closed = false;
    let mut is_truncated_seen = false;
    let mut next_token_seen = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let name = event.name();
                let name = name.as_ref();
                if xml_name_is(name, b"ListBucketResult") {
                    if root_seen || root_open || root_closed {
                        return Err(list_protocol_error(
                            "S3 ListObjectsV2 response must contain exactly one root",
                        ));
                    }
                    root_seen = true;
                    root_open = true;
                } else if !root_open || root_closed {
                    return Err(list_protocol_error(
                        "S3 ListObjectsV2 content appears outside its root",
                    ));
                } else if in_key || in_is_truncated || in_next_token {
                    return Err(list_protocol_error(
                        "S3 ListObjectsV2 scalar field contains nested XML",
                    ));
                } else if xml_name_is(name, b"Key") {
                    in_key = true;
                    key_text.clear();
                } else if xml_name_is(name, b"IsTruncated") {
                    if is_truncated_seen {
                        return Err(list_protocol_error(
                            "S3 ListObjectsV2 response repeats IsTruncated",
                        ));
                    }
                    in_is_truncated = true;
                    truncated_text.clear();
                } else if xml_name_is(name, b"NextContinuationToken") {
                    if next_token_seen {
                        return Err(list_protocol_error(
                            "S3 ListObjectsV2 response repeats NextContinuationToken",
                        ));
                    }
                    in_next_token = true;
                    next_token_text.clear();
                }
            }
            Ok(Event::Text(text)) if in_key => {
                key_text.push_str(&decoded_xml_text("S3 object key", &text)?);
            }
            Ok(Event::Text(text)) if in_is_truncated => {
                truncated_text.push_str(&decoded_xml_text("S3 IsTruncated", &text)?);
            }
            Ok(Event::Text(text)) if in_next_token => {
                next_token_text.push_str(&decoded_xml_text("S3 continuation token", &text)?);
            }
            Ok(Event::GeneralRef(reference)) if in_key => {
                key_text.push_str(&decoded_xml_ref("S3 object key", &reference)?);
            }
            Ok(Event::GeneralRef(reference)) if in_is_truncated => {
                truncated_text.push_str(&decoded_xml_ref("S3 IsTruncated", &reference)?);
            }
            Ok(Event::GeneralRef(reference)) if in_next_token => {
                next_token_text.push_str(&decoded_xml_ref("S3 continuation token", &reference)?);
            }
            Ok(Event::End(event)) => {
                let name = event.name();
                let name = name.as_ref();
                if xml_name_is(name, b"ListBucketResult") {
                    if !root_open || in_key || in_is_truncated || in_next_token {
                        return Err(list_protocol_error(
                            "S3 ListObjectsV2 root closes with incomplete content",
                        ));
                    }
                    root_open = false;
                    root_closed = true;
                } else if !root_open {
                    return Err(list_protocol_error(
                        "S3 ListObjectsV2 closing tag appears outside its root",
                    ));
                } else if xml_name_is(name, b"Key") {
                    if !in_key {
                        return Err(list_protocol_error(
                            "S3 ListObjectsV2 Key closing tag is unbalanced",
                        ));
                    }
                    in_key = false;
                    if let Some(path) = relative_path_from_key(prefix, &key_text)? {
                        page.paths.push(path);
                    }
                } else if xml_name_is(name, b"IsTruncated") {
                    if !in_is_truncated {
                        return Err(list_protocol_error(
                            "S3 ListObjectsV2 IsTruncated closing tag is unbalanced",
                        ));
                    }
                    in_is_truncated = false;
                    page.is_truncated = match truncated_text.trim() {
                        value if value.eq_ignore_ascii_case("true") => true,
                        value if value.eq_ignore_ascii_case("false") => false,
                        _ => {
                            return Err(list_protocol_error(
                                "S3 ListObjectsV2 IsTruncated is not boolean",
                            ));
                        }
                    };
                    is_truncated_seen = true;
                } else if xml_name_is(name, b"NextContinuationToken") {
                    if !in_next_token {
                        return Err(list_protocol_error(
                            "S3 ListObjectsV2 NextContinuationToken closing tag is unbalanced",
                        ));
                    }
                    in_next_token = false;
                    let token = next_token_text.trim().to_string();
                    if !token.is_empty() {
                        page.next_continuation_token = Some(token);
                    }
                    next_token_seen = true;
                }
            }
            Ok(Event::Empty(event)) => {
                let name = event.name();
                let name = name.as_ref();
                if !root_open
                    || xml_name_is(name, b"ListBucketResult")
                    || xml_name_is(name, b"Key")
                    || xml_name_is(name, b"IsTruncated")
                    || xml_name_is(name, b"NextContinuationToken")
                {
                    return Err(list_protocol_error(
                        "S3 ListObjectsV2 response contains an incomplete empty element",
                    ));
                }
            }
            Ok(Event::Text(text)) if !root_open && !text.iter().all(u8::is_ascii_whitespace) => {
                return Err(list_protocol_error(
                    "S3 ListObjectsV2 response contains text outside its root",
                ));
            }
            Ok(Event::Eof) => {
                if !root_seen
                    || !root_closed
                    || root_open
                    || in_key
                    || in_is_truncated
                    || in_next_token
                    || !is_truncated_seen
                {
                    return Err(list_protocol_error(
                        "S3 ListObjectsV2 response is empty or truncated",
                    ));
                }
                break;
            }
            Err(err) => {
                return Err(RemoteProjectionProviderError::ProviderIo(format!(
                    "failed to parse S3 ListObjectsV2 response: {err}"
                )));
            }
            _ => {}
        }
    }
    Ok(page)
}

fn relative_path_from_key(
    prefix: &str,
    key: &str,
) -> Result<Option<String>, RemoteProjectionProviderError> {
    let rel = if prefix.is_empty() {
        key
    } else {
        key.strip_prefix(prefix).ok_or_else(|| {
            list_protocol_error("S3 ListObjectsV2 returned an object outside the requested prefix")
        })?
    };
    if rel.is_empty() || rel.ends_with('/') {
        Ok(None)
    } else {
        Ok(Some(rel.to_string()))
    }
}

fn xml_name_is(name: &[u8], local: &[u8]) -> bool {
    name == local
        || name
            .strip_suffix(local)
            .is_some_and(|prefix| prefix.ends_with(b":"))
}

fn list_budget_error(message: impl Into<String>) -> RemoteProjectionProviderError {
    RemoteProjectionProviderError::ProviderIo(message.into())
}

fn list_protocol_error(message: impl Into<String>) -> RemoteProjectionProviderError {
    RemoteProjectionProviderError::ProviderIo(message.into())
}
