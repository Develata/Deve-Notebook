//! plan_ref:
//!   - 06_backup#remote-projection-transport-contract

use deve_core::remote_projection::RemoteProjectionProviderError;
use reqwest::Url;

pub(super) fn webdav_locator_to_https_url(
    locator: &str,
) -> Result<Url, RemoteProjectionProviderError> {
    let https = strip_locator_scheme(locator, "webdav+").ok_or_else(|| {
        RemoteProjectionProviderError::ProviderIo("WebDAV locator scheme is invalid".into())
    })?;
    Url::parse(https)
        .map_err(|err| RemoteProjectionProviderError::ProviderIo(format!("invalid URL: {err}")))
}

fn strip_locator_scheme<'a>(locator: &'a str, scheme: &str) -> Option<&'a str> {
    locator
        .get(..scheme.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(scheme))
        .then(|| locator.get(scheme.len()..))
        .flatten()
}

pub(super) fn webdav_collection_url(
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
        segments.pop_if_empty();
        for segment in path_segments {
            segments.push(segment);
        }
    }
    Ok(url)
}

pub(super) fn webdav_file_url(
    base: &Url,
    file_path: &str,
) -> Result<Url, RemoteProjectionProviderError> {
    let segments = file_path.split('/').collect::<Vec<_>>();
    webdav_collection_url(base, &segments)
}

pub(super) fn relative_path_from_href(
    base: &Url,
    href: &str,
) -> Result<Option<String>, RemoteProjectionProviderError> {
    let href = href.trim();
    if href.is_empty() {
        return Err(RemoteProjectionProviderError::InvalidProjectionPath);
    }
    let href_url = if let Ok(url) = Url::parse(href) {
        url
    } else if href.starts_with('/') {
        let mut origin = base.clone();
        origin.set_path("/");
        origin
            .join(href.trim_start_matches('/'))
            .map_err(invalid_href_error)?
    } else {
        base.join(href).map_err(invalid_href_error)?
    };
    if href_url.scheme() != base.scheme()
        || href_url.host_str() != base.host_str()
        || href_url.port_or_known_default() != base.port_or_known_default()
    {
        return Err(RemoteProjectionProviderError::ProviderIo(
            "WebDAV PROPFIND href is outside the admitted origin".into(),
        ));
    }
    if !href_url.username().is_empty()
        || href_url.password().is_some()
        || href_url.query().is_some()
        || href_url.fragment().is_some()
    {
        return Err(RemoteProjectionProviderError::InvalidProjectionPath);
    }

    let base_path = base.path().trim_end_matches('/');
    let href_path = href_url.path().trim_end_matches('/');
    if href_path == base_path {
        return Ok(None);
    }
    let prefix = if base_path.is_empty() || base_path == "/" {
        "/".to_string()
    } else {
        format!("{base_path}/")
    };
    let Some(rel) = href_path.strip_prefix(&prefix) else {
        return Err(RemoteProjectionProviderError::ProviderIo(
            "WebDAV PROPFIND href is outside the admitted collection".into(),
        ));
    };
    if rel.is_empty() {
        Ok(None)
    } else {
        decode_relative_href_path(rel).map(Some)
    }
}

fn decode_relative_href_path(rel: &str) -> Result<String, RemoteProjectionProviderError> {
    let mut decoded_segments = Vec::new();
    for segment in rel.split('/') {
        let segment = percent_decode_utf8(segment)?;
        if segment.is_empty() || segment == "." || segment == ".." || segment.contains(['/', '\\'])
        {
            return Err(RemoteProjectionProviderError::InvalidProjectionPath);
        }
        decoded_segments.push(segment);
    }
    Ok(decoded_segments.join("/"))
}

fn percent_decode_utf8(input: &str) -> Result<String, RemoteProjectionProviderError> {
    let input = input.as_bytes();
    let mut output = Vec::with_capacity(input.len());
    let mut index = 0usize;
    while index < input.len() {
        if input[index] == b'%' {
            let Some(hex) = input.get(index + 1..index + 3) else {
                return Err(RemoteProjectionProviderError::InvalidProjectionPath);
            };
            let high = decode_hex(hex[0])?;
            let low = decode_hex(hex[1])?;
            output.push((high << 4) | low);
            index += 3;
        } else {
            output.push(input[index]);
            index += 1;
        }
    }
    String::from_utf8(output).map_err(|_| RemoteProjectionProviderError::InvalidProjectionPath)
}

fn decode_hex(byte: u8) -> Result<u8, RemoteProjectionProviderError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(RemoteProjectionProviderError::InvalidProjectionPath),
    }
}

fn invalid_href_error(err: impl std::fmt::Display) -> RemoteProjectionProviderError {
    RemoteProjectionProviderError::ProviderIo(format!("invalid WebDAV href: {err}"))
}
