//! plan_ref: infra

#[cfg(windows)]
mod atomic_replace_windows;
mod authority;
mod file_lock;
mod identity;
#[cfg(windows)]
mod owner_only;
mod quarantine;

#[cfg_attr(target_arch = "wasm32", allow(unused_imports))]
pub(crate) use authority::{
    create_regular_file_lock_new, ensure_open_file_matches_identity,
    open_regular_file_lock_existing,
};
pub use file_lock::{FileTryLockError, lock_file_exclusive, try_lock_file_exclusive, unlock_file};
pub use identity::{HostFileIdentity, HostPathIdentity, HostPathKind, HostPathState};
#[cfg_attr(target_arch = "wasm32", allow(unused_imports))]
pub(crate) use quarantine::{HostQuarantineCut, HostQuarantinePlan, delete_pinned_identity};

use anyhow::{Context, Result};
use std::fs::{File, Metadata, OpenOptions};
use std::path::Path;

/// Creates one same-directory temporary file suitable for
/// [`replace_file_atomically`].
///
/// Windows handle-based rename requires `DELETE` access on the original
/// handle. Keeping that exact handle alive also prevents a pathname swap from
/// changing which file is published.
pub fn create_atomic_replace_temp(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    apply_atomic_temp_options(&mut options);
    let file = options.open(path)?;
    validate_regular_handle(&file, path, "atomic replacement temp")?;
    Ok(file)
}

/// Atomically replace one file within a pre-validated host-runtime directory.
///
/// The source must already be fully written and synced. Callers remain
/// responsible for syncing the containing directory after this returns.
#[cfg(not(windows))]
pub fn replace_file_atomically(
    source_file: &File,
    source: &Path,
    destination: &Path,
) -> std::io::Result<()> {
    ensure_open_file_matches_path(source_file, source, "atomic replacement temp")?;
    std::fs::rename(source, destination)
}

/// Windows equivalent of [`replace_file_atomically`].
#[cfg(windows)]
pub fn replace_file_atomically(
    source_file: &File,
    source: &Path,
    destination: &Path,
) -> std::io::Result<()> {
    atomic_replace_windows::replace_file_atomically(source_file, source, destination)
}

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

/// Creates one new regular file without following a final-component
/// symlink/reparse point and keeps the exact opened identity as a witness.
///
/// This is intended for authority files that another library subsequently
/// opens by pathname. Callers must keep the returned handle alive and use
/// [`ensure_open_file_matches_path`] after that second open.
pub fn create_regular_file_new(path: &Path, context: &str) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    apply_no_follow(&mut options);
    let file = options.open(path).map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!("Failed to create {context} without following links at {path:?}: {error}"),
        )
    })?;
    validate_regular_handle(&file, path, context)?;
    Ok(file)
}

/// Creates a new authority file with owner-only Unix permissions from the
/// first observable handle and without following a final-component link.
#[cfg(not(windows))]
pub fn create_owner_only_regular_file_new(path: &Path, context: &str) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    apply_no_follow(&mut options);
    let file = options.open(path).map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!("Failed to create {context} without following links at {path:?}: {error}"),
        )
    })?;
    validate_regular_handle(&file, path, context)?;
    Ok(file)
}

#[cfg(windows)]
pub fn create_owner_only_regular_file_new(path: &Path, context: &str) -> std::io::Result<File> {
    let file = owner_only::create_owner_only_regular_file_new(path, context)?;
    validate_regular_handle(&file, path, context)?;
    Ok(file)
}

/// Opens an existing authority file and enforces owner-only access on the
/// exact opened handle before any secret bytes are read.
pub fn open_owner_only_regular_file_read(path: &Path, context: &str) -> std::io::Result<File> {
    #[cfg(unix)]
    let file = {
        use std::os::unix::fs::PermissionsExt;
        let file = open_regular_file_read(path, context)?;
        if file.metadata()?.permissions().mode() & 0o777 != 0o600 {
            file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        file
    };
    #[cfg(windows)]
    let file = owner_only::open_owner_only_regular_file_read(path, context)?;
    #[cfg(not(any(unix, windows)))]
    let file = open_regular_file_read(path, context)?;
    validate_regular_handle(&file, path, context)?;
    ensure_open_file_matches_path(&file, path, context)?;
    Ok(file)
}

/// Opens or creates a regular lock file without following a final-component
/// symlink/reparse point. Callers should lock it, then call
/// [`ensure_open_file_matches_path`] before relying on the pathname as a mutex.
pub fn open_regular_file_lock(path: &Path, context: &str) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    apply_lock_options(&mut options);
    let file = options.open(path).map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!("Failed to open {context} without following links at {path:?}: {error}"),
        )
    })?;
    validate_regular_handle(&file, path, context)?;
    Ok(file)
}

#[cfg(windows)]
fn apply_lock_options(options: &mut OpenOptions) {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };
    options
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
}

#[cfg(not(windows))]
fn apply_lock_options(options: &mut OpenOptions) {
    apply_no_follow(options);
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
fn apply_atomic_temp_options(options: &mut OpenOptions) {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Foundation::GENERIC_WRITE;
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };
    options
        .access_mode(GENERIC_WRITE | DELETE)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
}

#[cfg(not(windows))]
fn apply_atomic_temp_options(options: &mut OpenOptions) {
    apply_no_follow(options);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn atomic_replace_publishes_the_exact_open_temp_handle() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let temp = dir.path().join("record.tmp");
        let destination = dir.path().join("record.json");
        std::fs::write(&destination, b"old")?;
        let mut file = create_atomic_replace_temp(&temp)?;
        file.write_all(b"new")?;
        file.sync_all()?;

        replace_file_atomically(&file, &temp, &destination)?;

        assert_eq!(std::fs::read(destination)?, b"new");
        assert!(!temp.exists());
        let mut names = std::fs::read_dir(dir.path())?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<std::io::Result<Vec<_>>>()?;
        names.sort();
        assert_eq!(names, vec![std::ffi::OsString::from("record.json")]);
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn atomic_replace_terminates_an_aligned_utf16_destination() -> std::io::Result<()> {
        use std::mem::{offset_of, size_of};
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_RENAME_INFO;

        let dir = tempfile::tempdir()?;
        let temp = dir.path().join("aligned.tmp");
        let destination = (0..size_of::<usize>())
            .map(|padding| {
                dir.path()
                    .join(format!("aligned-{}.json", "x".repeat(padding)))
            })
            .find(|path| {
                let wide_bytes = std::fs::canonicalize(dir.path())
                    .expect("canonical temp directory")
                    .join(path.file_name().expect("destination name"))
                    .as_os_str()
                    .encode_wide()
                    .count()
                    * size_of::<u16>();
                (offset_of!(FILE_RENAME_INFO, FileName) + wide_bytes)
                    .is_multiple_of(size_of::<usize>())
            })
            .expect("one short suffix must align the FILE_RENAME_INFO payload");
        let destination_name = destination
            .file_name()
            .expect("destination name")
            .to_os_string();
        let mut file = create_atomic_replace_temp(&temp)?;
        file.write_all(b"aligned")?;
        file.sync_all()?;

        replace_file_atomically(&file, &temp, &destination)?;

        assert_eq!(std::fs::read(&destination)?, b"aligned");
        let names = std::fs::read_dir(dir.path())?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<std::io::Result<Vec<_>>>()?;
        assert_eq!(names, vec![destination_name]);
        Ok(())
    }

    #[test]
    fn atomic_replace_rejects_a_swapped_source_path() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let temp = dir.path().join("record.tmp");
        let displaced = dir.path().join("displaced.tmp");
        let destination = dir.path().join("record.json");
        std::fs::write(&destination, b"old")?;
        let mut original = create_atomic_replace_temp(&temp)?;
        original.write_all(b"expected")?;
        original.sync_all()?;
        std::fs::rename(&temp, &displaced)?;
        std::fs::write(&temp, b"swapped")?;

        let error = replace_file_atomically(&original, &temp, &destination)
            .expect_err("swapped pathname must fail closed");

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(std::fs::read(destination)?, b"old");
        assert_eq!(std::fs::read(displaced)?, b"expected");
        Ok(())
    }

    #[test]
    fn existing_authority_open_never_creates_a_missing_path() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("authority.lock");

        let error = open_regular_file_lock_existing(&path, "test authority lock")
            .expect_err("existing-only open must reject a missing path");

        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        assert!(!path.exists());
        Ok(())
    }

    #[test]
    fn fresh_authority_create_never_reuses_an_existing_path() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("authority.lock");
        let first = create_regular_file_lock_new(&path, "test authority lock")?;

        let error = create_regular_file_lock_new(&path, "test authority lock")
            .expect_err("fresh create must reject an existing lock lineage");

        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        ensure_open_file_matches_path(&first, &path, "test authority lock")?;
        Ok(())
    }

    #[test]
    fn captured_identity_rejects_path_replacement() -> std::io::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("authority.lock");
        drop(create_regular_file_lock_new(&path, "test authority lock")?);
        let expected = HostPathIdentity::capture(&path, HostPathKind::RegularFile)?;
        let replacement_path = dir.path().join("replacement.lock");
        drop(create_regular_file_lock_new(
            &replacement_path,
            "replacement authority lock",
        )?);
        std::fs::remove_file(&path)?;
        std::fs::rename(&replacement_path, &path)?;
        let replacement = open_regular_file_lock_existing(&path, "replacement authority lock")?;

        let error =
            ensure_open_file_matches_identity(&replacement, &expected, "test authority lock")
                .expect_err("replacement object must not satisfy the captured lineage");

        assert_eq!(error.kind(), std::io::ErrorKind::Other);
        Ok(())
    }
}
