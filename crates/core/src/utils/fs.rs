//! plan_ref: infra

use anyhow::{Context, Result};
use std::fs::{File, Metadata, OpenOptions};
use std::path::Path;

pub fn checked_exists(path: &Path, context: &str) -> Result<bool> {
    path.try_exists()
        .with_context(|| format!("Failed to stat {}: {:?}", context, path))
}

/// Opens an existing regular file without following a final-component symlink
/// or Windows reparse point, then validates the opened handle itself.
pub fn open_regular_file_read(path: &Path, context: &str) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    apply_no_follow(&mut options);
    let file = options.open(path).map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!("Failed to open {context} without following links at {path:?}: {error}"),
        )
    })?;
    validate_regular_handle(&file, path, context)?;
    Ok(file)
}

/// Opens or creates a regular lock file without following a final-component
/// symlink/reparse point. Callers should lock it, then call
/// [`ensure_open_file_matches_path`] before relying on the pathname as a mutex.
pub fn open_regular_file_lock(path: &Path, context: &str) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    apply_no_follow(&mut options);
    let file = options.open(path).map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!("Failed to open {context} without following links at {path:?}: {error}"),
        )
    })?;
    validate_regular_handle(&file, path, context)?;
    Ok(file)
}

/// Revalidates that a locked/open handle is still the file named by `path`.
/// This closes the check-before-open swap; the containing directory remains a
/// separately protected host-runtime boundary.
pub fn ensure_open_file_matches_path(
    file: &File,
    path: &Path,
    context: &str,
) -> std::io::Result<()> {
    let current = open_regular_file_read(path, context)?;
    if !same_file_identity(file, &current)? {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Refusing replaced {context} path whose identity changed: {path:?}"),
        ));
    }
    Ok(())
}

#[cfg(unix)]
pub fn sync_directory(path: &Path) -> std::io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(windows)]
pub fn sync_directory(path: &Path) -> std::io::Result<()> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    };
    OpenOptions::new()
        .write(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?
        .sync_all()
}

#[cfg(not(any(unix, windows)))]
pub fn sync_directory(path: &Path) -> std::io::Result<()> {
    File::open(path)?.sync_all()
}

fn validate_regular_handle(file: &File, path: &Path, context: &str) -> std::io::Result<()> {
    let metadata = file.metadata().map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!("Failed to inspect open {context} handle at {path:?}: {error}"),
        )
    })?;
    if !metadata.is_file() || is_reparse(&metadata) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Refusing non-regular or reparse {context} handle: {path:?}"),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn apply_no_follow(options: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
}

#[cfg(windows)]
fn apply_no_follow(options: &mut OpenOptions) {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
    options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
}

#[cfg(not(any(unix, windows)))]
fn apply_no_follow(_options: &mut OpenOptions) {}

#[cfg(unix)]
fn same_file_identity(left: &File, right: &File) -> std::io::Result<bool> {
    use std::os::unix::fs::MetadataExt;
    let left = left.metadata()?;
    let right = right.metadata()?;
    Ok(left.dev() == right.dev() && left.ino() == right.ino())
}

#[cfg(windows)]
fn same_file_identity(left: &File, right: &File) -> std::io::Result<bool> {
    Ok(windows_file_identity(left)? == windows_file_identity(right)?)
}

#[cfg(windows)]
fn windows_file_identity(file: &File) -> std::io::Result<(u32, u64)> {
    use std::mem::MaybeUninit;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    let mut info = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
    // SAFETY: `file` owns a valid handle and `info` points to writable storage.
    let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle(), info.as_mut_ptr()) };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: the successful Win32 call initialized every field of the struct.
    let info = unsafe { info.assume_init() };
    let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    Ok((info.dwVolumeSerialNumber, index))
}

#[cfg(not(any(unix, windows)))]
fn same_file_identity(_left: &File, _right: &File) -> std::io::Result<bool> {
    Ok(false)
}

#[cfg(windows)]
fn is_reparse(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse(_metadata: &Metadata) -> bool {
    false
}
