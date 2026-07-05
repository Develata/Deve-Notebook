//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport

use deve_core::remote_projection::RemoteProjectionProviderError;
use reqwest::Url;

pub(super) fn s3_file_url(
    locator: &str,
    region: &str,
    file_path: &str,
) -> Result<Url, RemoteProjectionProviderError> {
    if locator.starts_with("s3://") {
        return s3_aws_file_url(locator, region, file_path);
    }
    if locator.starts_with("s3+https://") {
        return Err(RemoteProjectionProviderError::ProviderIo(
            "S3 custom endpoint requires explicit credential binding before provider I/O (provider_io_ready=false)".into(),
        ));
    }
    Err(RemoteProjectionProviderError::ProviderIo(
        "S3 locator scheme is invalid".into(),
    ))
}

fn s3_aws_file_url(
    locator: &str,
    region: &str,
    file_path: &str,
) -> Result<Url, RemoteProjectionProviderError> {
    let parsed = Url::parse(locator)
        .map_err(|err| RemoteProjectionProviderError::ProviderIo(format!("invalid URL: {err}")))?;
    let bucket = parsed.host_str().ok_or_else(|| {
        RemoteProjectionProviderError::ProviderIo("S3 locator has no bucket".into())
    })?;
    let mut url = Url::parse(&format!("https://{bucket}.s3.{region}.amazonaws.com/"))
        .map_err(|err| RemoteProjectionProviderError::ProviderIo(format!("invalid URL: {err}")))?;
    append_segments(&mut url, parsed.path().trim_start_matches('/').split('/'))?;
    append_segments(&mut url, file_path.split('/'))?;
    Ok(url)
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
