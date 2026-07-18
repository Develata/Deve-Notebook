//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract
//!
//! Strict XML text/entity decoding shared by the WebDAV and S3 listing
//! adapters. Provider listing data remains untrusted transport input.

use deve_core::remote_projection::RemoteProjectionProviderError;
use quick_xml::events::{BytesRef, BytesText};

pub(super) fn decoded_xml_text(
    label: &str,
    text: &BytesText<'_>,
) -> Result<String, RemoteProjectionProviderError> {
    text.xml10_content()
        .map(|value| value.into_owned())
        .map_err(|error| {
            RemoteProjectionProviderError::ProviderIo(format!("failed to decode {label}: {error}"))
        })
}

pub(super) fn decoded_xml_ref(
    label: &str,
    reference: &BytesRef<'_>,
) -> Result<String, RemoteProjectionProviderError> {
    let decoded = reference.decode().map_err(|error| {
        RemoteProjectionProviderError::ProviderIo(format!("failed to decode {label}: {error}"))
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
            let code = u32::from_str_radix(&numeric[2..], 16).map_err(|error| {
                RemoteProjectionProviderError::ProviderIo(format!(
                    "invalid XML character reference in {label}: {error}"
                ))
            })?;
            char::from_u32(code).ok_or_else(|| {
                RemoteProjectionProviderError::ProviderIo(format!(
                    "invalid XML character reference in {label}: {entity}"
                ))
            })?
        }
        numeric if numeric.starts_with('#') => {
            let code = numeric[1..].parse::<u32>().map_err(|error| {
                RemoteProjectionProviderError::ProviderIo(format!(
                    "invalid XML character reference in {label}: {error}"
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
