//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!
//! Shared bounded text-file admission for plugin-owned resources.

use std::fs::File;
use std::io::{Read, Take};
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) const MAX_PLUGIN_MANIFEST_BYTES: u64 = 64 * 1024;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const MAX_PLUGIN_SCRIPT_BYTES: u64 = 1024 * 1024;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const MAX_PLUGIN_HOST_TEXT_BYTES: u64 = 1024 * 1024;
pub(crate) const MAX_SKILL_FILE_BYTES: u64 = 256 * 1024;
pub(crate) const MAX_SKILL_TOTAL_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const MAX_SKILL_COUNT: usize = 128;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const MAX_PLUGIN_COUNT: usize = 64;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn read_utf8_file_bounded(
    path: &Path,
    max_bytes: u64,
    context: &str,
) -> std::io::Result<String> {
    let file = crate::utils::fs::open_regular_file_read(path, context)?;
    read_utf8_handle_bounded_and_verify(file, path, max_bytes, context)
}

pub(crate) fn read_utf8_handle_bounded_and_verify(
    mut file: File,
    path: &Path,
    max_bytes: u64,
    context: &str,
) -> std::io::Result<String> {
    let text = read_utf8_handle_bounded_inner(&mut file, max_bytes, context)?;
    crate::utils::fs::ensure_open_file_matches_path(&file, path, context)?;
    Ok(text)
}

fn read_utf8_handle_bounded_inner(
    file: &mut File,
    max_bytes: u64,
    context: &str,
) -> std::io::Result<String> {
    let metadata = file.metadata()?;
    if metadata.len() > max_bytes {
        return Err(limit_error(context, max_bytes));
    }

    let capacity = usize::try_from(metadata.len()).unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    let mut bounded: Take<_> = file.take(max_bytes.saturating_add(1));
    bounded.read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > max_bytes {
        return Err(limit_error(context, max_bytes));
    }
    String::from_utf8(bytes).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{context} must contain valid UTF-8"),
        )
    })
}

fn limit_error(context: &str, max_bytes: u64) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("{context} exceeds the {max_bytes}-byte resource budget"),
    )
}

#[cfg(test)]
mod tests {
    use super::read_utf8_file_bounded;

    #[test]
    fn bounded_reader_rejects_oversize_before_returning_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("oversize.txt");
        std::fs::write(&path, b"12345").expect("write fixture");

        let error = read_utf8_file_bounded(&path, 4, "fixture")
            .expect_err("oversize input must fail closed");

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("resource budget"));
    }
}
