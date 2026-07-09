//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use deve_core::remote_projection::RemoteProjectionProviderError;
use reqwest::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct S3CustomEndpointUrlBinding {
    endpoint_origin: Url,
    bucket: String,
}

impl S3CustomEndpointUrlBinding {
    pub(super) fn new(
        endpoint_origin: Url,
        bucket: impl Into<String>,
    ) -> Result<Self, RemoteProjectionProviderError> {
        let bucket = bucket.into();
        if bucket.trim().is_empty() || bucket.contains('/') {
            return Err(RemoteProjectionProviderError::ProviderIo(
                "S3 custom endpoint profile bucket is invalid".into(),
            ));
        }
        Ok(Self {
            endpoint_origin,
            bucket,
        })
    }
}

pub(super) fn reject_custom_https_endpoint_without_binding(
    locator: &str,
) -> Result<(), RemoteProjectionProviderError> {
    if is_s3_custom_https_locator(locator) {
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
    s3_file_url_with_binding(locator, region, file_path, None)
}

pub(super) fn s3_file_url_with_binding(
    locator: &str,
    region: &str,
    file_path: &str,
    custom_binding: Option<&S3CustomEndpointUrlBinding>,
) -> Result<Url, RemoteProjectionProviderError> {
    if is_s3_aws_locator(locator) {
        return s3_aws_file_url(locator, region, file_path);
    }
    if is_s3_custom_https_locator(locator) {
        let Some(binding) = custom_binding else {
            return Err(custom_endpoint_requires_binding_error());
        };
        return s3_custom_file_url(locator, binding, file_path);
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
    s3_list_url_with_binding(locator, region, continuation_token, None)
}

pub(super) fn s3_list_url_with_binding(
    locator: &str,
    region: &str,
    continuation_token: Option<&str>,
    custom_binding: Option<&S3CustomEndpointUrlBinding>,
) -> Result<Url, RemoteProjectionProviderError> {
    if is_s3_aws_locator(locator) {
        return s3_aws_list_url(locator, region, continuation_token);
    }
    if is_s3_custom_https_locator(locator) {
        let Some(binding) = custom_binding else {
            return Err(custom_endpoint_requires_binding_error());
        };
        return s3_custom_list_url(locator, binding, continuation_token);
    }
    Err(RemoteProjectionProviderError::ProviderIo(
        "S3 locator scheme is invalid".into(),
    ))
}

pub(super) fn s3_locator_prefix(locator: &str) -> Result<String, RemoteProjectionProviderError> {
    s3_locator_prefix_with_binding(locator, None)
}

pub(super) fn s3_locator_prefix_with_binding(
    locator: &str,
    custom_binding: Option<&S3CustomEndpointUrlBinding>,
) -> Result<String, RemoteProjectionProviderError> {
    if is_s3_aws_locator(locator) {
        let parsed = parse_s3_locator(locator)?;
        return Ok(locator_prefix(&parsed));
    }
    if is_s3_custom_https_locator(locator) {
        if custom_binding.is_none() {
            return Err(custom_endpoint_requires_binding_error());
        }
        let parsed = parse_s3_locator(locator)?;
        let (_bucket, prefix) = custom_locator_bucket_and_prefix(&parsed)?;
        return Ok(prefix);
    }
    Err(RemoteProjectionProviderError::ProviderIo(
        "S3 locator scheme is invalid".into(),
    ))
}

fn is_s3_aws_locator(locator: &str) -> bool {
    locator_has_scheme(locator, "s3://")
}

pub(super) fn is_s3_custom_https_locator(locator: &str) -> bool {
    locator_has_scheme(locator, "s3+https://")
}

fn locator_has_scheme(locator: &str, scheme: &str) -> bool {
    locator
        .get(..scheme.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(scheme))
}

pub(super) fn custom_endpoint_requires_binding_error() -> RemoteProjectionProviderError {
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

fn s3_custom_file_url(
    locator: &str,
    binding: &S3CustomEndpointUrlBinding,
    file_path: &str,
) -> Result<Url, RemoteProjectionProviderError> {
    let parsed = parse_s3_locator(locator)?;
    let (_bucket, prefix) = custom_locator_bucket_and_prefix(&parsed)?;
    let mut url = binding.endpoint_origin.clone();
    append_segments(&mut url, std::iter::once(binding.bucket.as_str()))?;
    append_segments(&mut url, prefix.trim_end_matches('/').split('/'))?;
    append_segments(&mut url, file_path.split('/'))?;
    Ok(url)
}

fn s3_custom_list_url(
    locator: &str,
    binding: &S3CustomEndpointUrlBinding,
    continuation_token: Option<&str>,
) -> Result<Url, RemoteProjectionProviderError> {
    let parsed = parse_s3_locator(locator)?;
    let (_bucket, prefix) = custom_locator_bucket_and_prefix(&parsed)?;
    let mut url = binding.endpoint_origin.clone();
    append_segments(&mut url, std::iter::once(binding.bucket.as_str()))?;
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

pub(super) fn custom_locator_origin_bucket_prefix(
    locator: &str,
) -> Result<(String, String, String), RemoteProjectionProviderError> {
    let parsed = parse_s3_locator(locator)?;
    let origin = custom_locator_https_origin(&parsed)?;
    let (bucket, prefix) = custom_locator_bucket_and_prefix(&parsed)?;
    Ok((origin, bucket, prefix))
}

fn custom_locator_https_origin(parsed: &Url) -> Result<String, RemoteProjectionProviderError> {
    if !is_s3_custom_https_locator(parsed.as_str()) {
        return Err(RemoteProjectionProviderError::ProviderIo(
            "S3 custom endpoint locator scheme is invalid".into(),
        ));
    }
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(RemoteProjectionProviderError::ProviderIo(
            "S3 custom endpoint locator must not contain credentials, query, or fragment data"
                .into(),
        ));
    }
    let host = parsed.host_str().ok_or_else(|| {
        RemoteProjectionProviderError::ProviderIo("S3 custom endpoint locator has no host".into())
    })?;
    Ok(match parsed.port() {
        Some(port) => format!("https://{host}:{port}"),
        None => format!("https://{host}"),
    })
}

fn custom_locator_bucket_and_prefix(
    parsed: &Url,
) -> Result<(String, String), RemoteProjectionProviderError> {
    let mut segments = parsed.path().trim_start_matches('/').split('/');
    let bucket = segments
        .next()
        .filter(|segment| !segment.is_empty())
        .ok_or_else(|| {
            RemoteProjectionProviderError::ProviderIo(
                "S3 custom endpoint locator has no bucket segment".into(),
            )
        })?
        .to_string();
    if bucket == "." || bucket == ".." || bucket.contains(':') {
        return Err(RemoteProjectionProviderError::ProviderIo(
            "S3 custom endpoint locator bucket segment is invalid".into(),
        ));
    }
    let prefix = locator_prefix_from_segments(segments)?;
    Ok((bucket, prefix))
}

fn locator_prefix_from_segments<'a>(
    segments: impl IntoIterator<Item = &'a str>,
) -> Result<String, RemoteProjectionProviderError> {
    let mut parts = Vec::new();
    for segment in segments {
        if segment.is_empty() {
            continue;
        }
        if matches!(segment, "." | "..") || segment.contains(':') {
            return Err(RemoteProjectionProviderError::ProviderIo(
                "S3 custom endpoint locator prefix segment is invalid".into(),
            ));
        }
        parts.push(segment);
    }
    if parts.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!("{}/", parts.join("/")))
    }
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
