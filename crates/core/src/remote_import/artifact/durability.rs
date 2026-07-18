//! plan_ref:
//!   - 03_storage/index#remote-import-runtime-layout
//!   - 06_backup#remote-import-state-machine
//!
//! Filesystem publication cuts that must become durable before the Redb workflow CAS.

use crate::remote_import::error::{RemoteImportError, RemoteImportResult};
use std::path::Path;

pub(super) fn sync_directory_checked(path: &Path) -> RemoteImportResult<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || is_reparse(&metadata) {
        return Err(RemoteImportError::UnsafeArtifactRoot(format!(
            "refusing to sync non-directory artifact path {:?}",
            path
        )));
    }
    sync_directory_platform(path)?;
    Ok(())
}

#[cfg(windows)]
fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse(_metadata: &std::fs::Metadata) -> bool {
    false
}

pub(super) fn publish_directory_no_replace(
    staging: &Path,
    final_path: &Path,
) -> RemoteImportResult<()> {
    require_absent(final_path)?;
    move_no_replace_platform(staging, final_path)?;
    sync_parent(final_path)
}

pub(super) fn publish_file_no_replace(temp: &Path, final_path: &Path) -> RemoteImportResult<()> {
    require_absent(final_path)?;
    move_file_no_replace_platform(temp, final_path)?;
    sync_parent(final_path)
}

pub(super) fn sync_parent(path: &Path) -> RemoteImportResult<()> {
    let parent = path.parent().ok_or_else(|| {
        RemoteImportError::UnsafeArtifactRoot(format!(
            "artifact path has no parent directory: {:?}",
            path
        ))
    })?;
    sync_directory_checked(parent)
}

fn require_absent(path: &Path) -> RemoteImportResult<()> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("artifact publication target already exists: {path:?}"),
        )
        .into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(unix)]
fn sync_directory_platform(path: &Path) -> std::io::Result<()> {
    std::fs::File::open(path)?.sync_all()
}

#[cfg(windows)]
fn sync_directory_platform(path: &Path) -> std::io::Result<()> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    };

    std::fs::OpenOptions::new()
        .write(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?
        .sync_all()
}

#[cfg(not(any(unix, windows)))]
fn sync_directory_platform(path: &Path) -> std::io::Result<()> {
    std::fs::File::open(path)?.sync_all()
}

#[cfg(windows)]
fn move_no_replace_platform(source: &Path, destination: &Path) -> std::io::Result<()> {
    move_write_through_windows(source, destination)
}

#[cfg(not(windows))]
fn move_no_replace_platform(source: &Path, destination: &Path) -> std::io::Result<()> {
    std::fs::rename(source, destination)
}

#[cfg(windows)]
fn move_file_no_replace_platform(source: &Path, destination: &Path) -> std::io::Result<()> {
    move_write_through_windows(source, destination)
}

#[cfg(not(windows))]
fn move_file_no_replace_platform(source: &Path, destination: &Path) -> std::io::Result<()> {
    std::fs::hard_link(source, destination)?;
    std::fs::remove_file(source)
}

#[cfg(windows)]
fn move_write_through_windows(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};

    let source = std::fs::canonicalize(source)?;
    let destination_parent = destination.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "artifact destination has no parent",
        )
    })?;
    let destination_name = destination.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "artifact destination has no file name",
        )
    })?;
    let destination = std::fs::canonicalize(destination_parent)?.join(destination_name);
    let source_wide = source
        .as_os_str()
        .encode_wide()
        .chain(once(0))
        .collect::<Vec<_>>();
    let destination_wide = destination
        .as_os_str()
        .encode_wide()
        .chain(once(0))
        .collect::<Vec<_>>();
    // SAFETY: both buffers are owned, NUL-terminated UTF-16 paths and live through the call.
    let moved = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_sync_and_file_publication_are_real_filesystem_operations() {
        let dir = tempfile::tempdir().expect("tempdir");
        let temp = dir.path().join("temp");
        let final_path = dir.path().join("final");
        std::fs::write(&temp, b"alpha").expect("temp file");
        std::fs::OpenOptions::new()
            .write(true)
            .open(&temp)
            .expect("open temp")
            .sync_all()
            .expect("sync temp");

        publish_file_no_replace(&temp, &final_path).expect("publish file");

        assert_eq!(std::fs::read(&final_path).expect("read final"), b"alpha");
        assert!(!temp.exists());
        sync_directory_checked(dir.path()).expect("sync directory");
    }

    #[test]
    fn publication_never_replaces_existing_file_or_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file_temp = dir.path().join("file-temp");
        let file_final = dir.path().join("file-final");
        std::fs::write(&file_temp, b"new").expect("temp file");
        std::fs::write(&file_final, b"old").expect("final file");
        let error = publish_file_no_replace(&file_temp, &file_final)
            .expect_err("existing file must not be replaced");
        assert!(
            matches!(error, RemoteImportError::Io(ref io) if io.kind() == std::io::ErrorKind::AlreadyExists)
        );
        assert_eq!(std::fs::read(&file_final).expect("read final"), b"old");

        let dir_temp = dir.path().join("dir-temp");
        let dir_final = dir.path().join("dir-final");
        std::fs::create_dir(&dir_temp).expect("temp directory");
        std::fs::create_dir(&dir_final).expect("final directory");
        let error = publish_directory_no_replace(&dir_temp, &dir_final)
            .expect_err("existing directory must not be replaced");
        assert!(
            matches!(error, RemoteImportError::Io(ref io) if io.kind() == std::io::ErrorKind::AlreadyExists)
        );
        assert!(dir_temp.is_dir());
        assert!(dir_final.is_dir());
    }
}
