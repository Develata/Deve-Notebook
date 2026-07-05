//! plan_ref:
//!   - 06_backup#backup-provider-dispatch-contract

use anyhow::Context;
use deve_core::backup::BackupLocator;
use reqwest::Url;

pub(super) fn webdav_endpoint(locator: &BackupLocator) -> anyhow::Result<Url> {
    let endpoint = locator
        .endpoint
        .as_deref()
        .context("Backup WebDAV endpoint is missing")?;
    Url::parse(endpoint).context("Backup WebDAV endpoint URL is invalid")
}

pub(super) fn webdav_collection_url(base: &Url, path_segments: &[&str]) -> anyhow::Result<Url> {
    let mut url = base.clone();
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| anyhow::anyhow!("Backup WebDAV endpoint cannot accept path segments"))?;
        for segment in path_segments {
            if !segment.is_empty() {
                segments.push(segment);
            }
        }
    }
    Ok(url)
}

pub(super) fn webdav_object_url(base: &Url, object_path: &str) -> anyhow::Result<Url> {
    let segments = object_path.split('/').collect::<Vec<_>>();
    webdav_collection_url(base, &segments)
}
