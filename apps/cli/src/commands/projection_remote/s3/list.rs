//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use super::credentials::S3Credentials;
use super::signing::signed_get_request;
use super::transport::S3Transport;
use super::url::{s3_list_url, s3_locator_prefix};
use crate::commands::projection_remote::collect::{is_markdown_path, is_reserved_projection_path};
use chrono::{DateTime, Utc};
use deve_core::remote_projection::{RemoteProjectionFile, RemoteProjectionProviderError};
use quick_xml::Reader;
use quick_xml::events::{BytesRef, BytesText, Event};
use std::collections::BTreeSet;

pub(super) const MAX_PULL_FILES: usize = 2_048;
const MAX_LIST_BODY_BYTES: usize = 4 * 1024 * 1024;
const MAX_LIST_PAGES: usize = 2_048;

pub(super) fn discover_remote_markdown_files<T: S3Transport>(
    transport: &T,
    credentials: &S3Credentials,
    region: &str,
    now: fn() -> DateTime<Utc>,
    locator: &str,
) -> Result<Vec<String>, RemoteProjectionProviderError> {
    let prefix = s3_locator_prefix(locator)?;
    let mut continuation_token = None;
    let mut pages = 0usize;
    let mut files = BTreeSet::new();
    loop {
        pages += 1;
        if pages > MAX_LIST_PAGES {
            return Err(list_budget_error(format!(
                "S3 pull exceeds list page budget of {MAX_LIST_PAGES}"
            )));
        }
        let list_url = s3_list_url(locator, region, continuation_token.as_deref())?;
        let response = transport.get(signed_get_request(
            list_url,
            credentials,
            region,
            now(),
            MAX_LIST_BODY_BYTES,
        )?)?;
        if !response.status.is_success() {
            return Err(RemoteProjectionProviderError::ProviderIo(format!(
                "S3 LIST failed with status {}",
                response.status.as_u16()
            )));
        }
        let page = parse_list_objects_v2_page(&response.body, &prefix)?;
        if page.is_truncated && page.next_continuation_token.is_none() {
            return Err(RemoteProjectionProviderError::ProviderIo(
                "S3 ListObjectsV2 response is truncated without a continuation token".into(),
            ));
        }
        for path in page.paths {
            if is_reserved_projection_path(&path) || !is_markdown_path(&path) {
                continue;
            }
            RemoteProjectionFile::new(&path, Vec::new())?;
            files.insert(path);
            if files.len() > MAX_PULL_FILES {
                return Err(list_budget_error(format!(
                    "S3 pull exceeds file budget of {MAX_PULL_FILES}"
                )));
            }
        }
        continuation_token = page.next_continuation_token;
        if continuation_token.is_none() {
            break;
        }
    }
    Ok(files.into_iter().collect())
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

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let name = event.name();
                let name = name.as_ref();
                if xml_name_is(name, b"Key") {
                    in_key = true;
                    key_text.clear();
                } else if xml_name_is(name, b"IsTruncated") {
                    in_is_truncated = true;
                    truncated_text.clear();
                } else if xml_name_is(name, b"NextContinuationToken") {
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
                if xml_name_is(name, b"Key") {
                    in_key = false;
                    if let Some(path) = relative_path_from_key(prefix, &key_text) {
                        page.paths.push(path);
                    }
                } else if xml_name_is(name, b"IsTruncated") {
                    in_is_truncated = false;
                    page.is_truncated = truncated_text.trim().eq_ignore_ascii_case("true");
                } else if xml_name_is(name, b"NextContinuationToken") {
                    in_next_token = false;
                    let token = next_token_text.trim().to_string();
                    if !token.is_empty() {
                        page.next_continuation_token = Some(token);
                    }
                }
            }
            Ok(Event::Eof) => break,
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

fn decoded_xml_text(
    label: &str,
    text: &BytesText<'_>,
) -> Result<String, RemoteProjectionProviderError> {
    text.xml_content()
        .map(|value| value.into_owned())
        .map_err(|err| {
            RemoteProjectionProviderError::ProviderIo(format!("failed to decode {label}: {err}"))
        })
}

fn decoded_xml_ref(
    label: &str,
    reference: &BytesRef<'_>,
) -> Result<String, RemoteProjectionProviderError> {
    let decoded = reference.decode().map_err(|err| {
        RemoteProjectionProviderError::ProviderIo(format!("failed to decode {label}: {err}"))
    })?;
    resolve_xml_entity(label, decoded.trim())
}

fn resolve_xml_entity(label: &str, entity: &str) -> Result<String, RemoteProjectionProviderError> {
    let resolved = match entity {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        numeric if numeric.starts_with("#x") => {
            let code = u32::from_str_radix(&numeric[2..], 16).map_err(|err| {
                RemoteProjectionProviderError::ProviderIo(format!(
                    "invalid XML character reference in {label}: {err}"
                ))
            })?;
            char::from_u32(code).ok_or_else(|| {
                RemoteProjectionProviderError::ProviderIo(format!(
                    "invalid XML character reference in {label}: {entity}"
                ))
            })?
        }
        numeric if numeric.starts_with('#') => {
            let code = numeric[1..].parse::<u32>().map_err(|err| {
                RemoteProjectionProviderError::ProviderIo(format!(
                    "invalid XML character reference in {label}: {err}"
                ))
            })?;
            char::from_u32(code).ok_or_else(|| {
                RemoteProjectionProviderError::ProviderIo(format!(
                    "invalid XML character reference in {label}: {entity}"
                ))
            })?
        }
        _ => {
            return Err(RemoteProjectionProviderError::ProviderIo(format!(
                "unsupported XML entity in {label}: {entity}"
            )));
        }
    };
    Ok(resolved.to_string())
}

fn relative_path_from_key(prefix: &str, key: &str) -> Option<String> {
    let rel = if prefix.is_empty() {
        key
    } else {
        key.strip_prefix(prefix)?
    };
    if rel.is_empty() || rel.ends_with('/') {
        None
    } else {
        Some(rel.to_string())
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
