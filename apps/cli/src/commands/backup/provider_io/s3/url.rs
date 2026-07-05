//! plan_ref:
//!   - 06_backup#backup-provider-dispatch-contract

use anyhow::{Context, bail};
use deve_core::backup::{BackupLocator, BackupProviderKind};
use reqwest::Url;

pub(super) fn s3_pack_url(
    locator: &BackupLocator,
    region: &str,
    object_path: &str,
) -> anyhow::Result<Url> {
    match locator.provider {
        BackupProviderKind::S3 => {
            let mut url = Url::parse(&format!(
                "https://{}.s3.{region}.amazonaws.com/",
                locator.namespace
            ))
            .context("Backup S3 URL is invalid")?;
            append_segments(&mut url, object_path.split('/'))?;
            Ok(url)
        }
        BackupProviderKind::S3CompatibleHttps => {
            let endpoint = locator
                .endpoint
                .as_deref()
                .context("Backup S3-compatible endpoint is missing")?;
            let mut url =
                Url::parse(endpoint).context("Backup S3-compatible endpoint is invalid")?;
            append_segments(
                &mut url,
                std::iter::once(locator.namespace.as_str()).chain(object_path.split('/')),
            )?;
            Ok(url)
        }
        BackupProviderKind::WebDavHttps => bail!("Backup S3 uploader received WebDAV locator"),
    }
}

fn append_segments<'a>(
    url: &mut Url,
    segments: impl IntoIterator<Item = &'a str>,
) -> anyhow::Result<()> {
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|_| anyhow::anyhow!("Backup S3 URL cannot accept path segments"))?;
        for segment in segments {
            if !segment.is_empty() {
                path.push(segment);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s3_compatible_upload_uses_bucket_namespace_under_endpoint() {
        let locator = BackupLocator::parse("s3+https://s3.example.com/bucket-name/deve").unwrap();
        let target = s3_pack_url(
            &locator,
            "us-east-1",
            "deve/branches/writer-1/packs/000001.pack.enc",
        )
        .unwrap();

        assert_eq!(
            target.as_str(),
            "https://s3.example.com/bucket-name/deve/branches/writer-1/packs/000001.pack.enc"
        );
    }
}
