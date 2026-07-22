//! plan_ref:
//!   - 03_storage/authority#local-authority-owner-contract
//!
//! Existing-only and identity-bound file helpers for host authority owners.

use super::{HostPathIdentity, apply_no_follow, create_regular_file_new, validate_regular_handle};
use std::fs::{File, OpenOptions};
use std::path::Path;

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn open_regular_file_read_write_existing(
    path: &Path,
    context: &str,
) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    apply_no_follow(&mut options);
    let file = options.open(path).map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!(
                "Failed to open existing {context} without following links at {path:?}: {error}"
            ),
        )
    })?;
    validate_regular_handle(&file, path, context)?;
    Ok(file)
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn open_regular_file_lock_existing(path: &Path, context: &str) -> std::io::Result<File> {
    open_regular_file_read_write_existing(path, context)
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn create_regular_file_lock_new(path: &Path, context: &str) -> std::io::Result<File> {
    create_regular_file_new(path, context)
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn ensure_open_file_matches_identity(
    file: &File,
    expected: &HostPathIdentity,
    context: &str,
) -> std::io::Result<()> {
    if expected.matches_open_file(file)? {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "Opened {context} no longer matches its captured identity at {:?}",
            expected.path()
        )))
    }
}
