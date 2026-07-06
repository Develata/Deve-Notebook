//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use deve_core::remote_projection::RemoteProjectionProviderError;
use reqwest::Url;

pub(super) fn reject_custom_https_endpoint_without_binding(
    locator: &str,
) -> Result<(), RemoteProjectionProviderError> {
    if locator.starts_with("s3+https://") {
        Err(custom_endpoint_requires_binding_error())
    } else {
        Ok(())
    }
}

pub(super) fn s3_file_url(
    locator: &str,
    region: &str,
    file_path: &str,
) -> Result<Url, RemoteProjectionProviderError> {
    if locator.starts_with("s3://") {
        return s3_aws_file_url(locator, region, file_path);
    }
    if locator.starts_with("s3+https://") {
        return Err(custom_endpoint_requires_binding_error());
    }
    Err(RemoteProjectionProviderError::ProviderIo(
        "S3 locator scheme is invalid".into(),
    ))
}

pub(super) fn s3_list_url(
    locator: &str,
    region: &str,
    continuation_token: Option<&str>,
) -> Result<Url, RemoteProjectionProviderError> {
    if locator.starts_with("s3://") {
        return s3_aws_list_url(locator, region, continuation_token);
    }
    if locator.starts_with("s3+https://") {
        return Err(custom_endpoint_requires_binding_error());
    }
    Err(RemoteProjectionProviderError::ProviderIo(
        "S3 locator scheme is invalid".into(),
    ))
}

pub(super) fn s3_locator_prefix(locator: &str) -> Result<String, RemoteProjectionProviderError> {
    if locator.starts_with("s3://") {
        let parsed = parse_s3_locator(locator)?;
        return Ok(locator_prefix(&parsed));
    }
    if locator.starts_with("s3+https://") {
        return Err(custom_endpoint_requires_binding_error());
    }
    Err(RemoteProjectionProviderError::ProviderIo(
        "S3 locator scheme is invalid".into(),
    ))
}

fn custom_endpoint_requires_binding_error() -> RemoteProjectionProviderError {
    RemoteProjectionProviderError::ProviderIo(
        "S3 custom endpoint requires explicit credential binding before provider I/O (provider_io_ready=false)".into(),
    )
}

fn s3_aws_file_url(
    locator: &str,
    region: &str,
    file_path: &str,
) -> Result<Url, RemoteProjectionProviderError> {
    let parsed = parse_s3_locator(locator)?;
    let bucket = locator_bucket(&parsed)?;
    let mut url = Url::parse(&format!("https://{bucket}.s3.{region}.amazonaws.com/"))
        .map_err(|err| RemoteProjectionProviderError::ProviderIo(format!("invalid URL: {err}")))?;
    append_segments(&mut url, parsed.path().trim_start_matches('/').split('/'))?;
    append_segments(&mut url, file_path.split('/'))?;
    Ok(url)
}

fn s3_aws_list_url(
    locator: &str,
    region: &str,
    continuation_token: Option<&str>,
) -> Result<Url, RemoteProjectionProviderError> {
    let parsed = parse_s3_locator(locator)?;
    let bucket = locator_bucket(&parsed)?;
    let mut url = Url::parse(&format!("https://{bucket}.s3.{region}.amazonaws.com/"))
        .map_err(|err| RemoteProjectionProviderError::ProviderIo(format!("invalid URL: {err}")))?;
    let prefix = locator_prefix(&parsed);
    let mut query = Vec::new();
    if let Some(token) = continuation_token {
        query.push(format!("continuation-token={}", aws_query_encode(token)));
    }
    query.push("list-type=2".to_string());
    query.push(format!("prefix={}", aws_query_encode(&prefix)));
    url.set_query(Some(&query.join("&")));
    Ok(url)
}

fn parse_s3_locator(locator: &str) -> Result<Url, RemoteProjectionProviderError> {
    Url::parse(locator)
        .map_err(|err| RemoteProjectionProviderError::ProviderIo(format!("invalid URL: {err}")))
}

fn locator_bucket(parsed: &Url) -> Result<&str, RemoteProjectionProviderError> {
    parsed
        .host_str()
        .ok_or_else(|| RemoteProjectionProviderError::ProviderIo("S3 locator has no bucket".into()))
}

fn locator_prefix(parsed: &Url) -> String {
    let prefix = parsed.path().trim_start_matches('/').trim_end_matches('/');
    if prefix.is_empty() {
        String::new()
    } else {
        format!("{prefix}/")
    }
}

fn append_segments<'a>(
    url: &mut Url,
    segments: impl IntoIterator<Item = &'a str>,
) -> Result<(), RemoteProjectionProviderError> {
    {
        let mut path = url.path_segments_mut().map_err(|_| {
            RemoteProjectionProviderError::ProviderIo(
                "S3 locator cannot be a base URL for path segments".into(),
            )
        })?;
        for segment in segments {
            if !segment.is_empty() {
                path.push(segment);
            }
        }
    }
    Ok(())
}

fn aws_query_encode(input: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::new();
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(HEX[(byte >> 4) as usize] as char);
            encoded.push(HEX[(byte & 0x0f) as usize] as char);
        }
    }
    encoded
}
