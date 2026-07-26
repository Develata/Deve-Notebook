//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! Descriptor-relative quarantine operations for Unix hosts.

use super::invalid;
use crate::utils::fs::identity::{identity_from_handle, open_no_follow};
use crate::utils::fs::{HostFileIdentity, HostPathIdentity, HostPathKind};
use std::ffi::{CStr, CString};
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub(super) fn native_rename_no_replace(
    original: &HostPathIdentity,
    destination: &Path,
    destination_parent_identity: HostFileIdentity,
) -> std::io::Result<()> {
    let source_parent = open_exact_parent(original.path(), original.parent_identity())?;
    let destination_parent = open_exact_parent(destination, destination_parent_identity)?;
    let source_name = component_cstring(original.path())?;
    let destination_name = component_cstring(destination)?;
    let source = openat_no_follow(&source_parent, &source_name, original.kind())?;
    if identity_from_handle(&source)? != original.object_identity() {
        return Err(invalid("quarantine source handle identity changed"));
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    let result = unsafe {
        libc::renameat2(
            source_parent.as_raw_fd(),
            source_name.as_ptr(),
            destination_parent.as_raw_fd(),
            destination_name.as_ptr(),
            libc::RENAME_NOREPLACE as _,
        )
    };
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::renameatx_np(
            source_parent.as_raw_fd(),
            source_name.as_ptr(),
            destination_parent.as_raw_fd(),
            destination_name.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error());
    }
    source_parent.sync_all()?;
    if source_parent.as_raw_fd() != destination_parent.as_raw_fd() {
        destination_parent.sync_all()?;
    }
    Ok(())
}

pub(super) fn delete_file_pinned(identity: &HostPathIdentity) -> std::io::Result<()> {
    let parent = open_exact_parent(identity.path(), identity.parent_identity())?;
    let name = component_cstring(identity.path())?;
    let file = openat_no_follow(&parent, &name, HostPathKind::RegularFile)?;
    if identity_from_handle(&file)? != identity.object_identity()
        || fstatat_identity(&parent, &name)? != identity.object_identity()
    {
        return Err(invalid("quarantine file identity changed before unlink"));
    }
    if unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    parent.sync_all()
}

pub(super) fn delete_directory_pinned(identity: &HostPathIdentity) -> std::io::Result<()> {
    let parent = open_exact_parent(identity.path(), identity.parent_identity())?;
    let name = component_cstring(identity.path())?;
    let directory = openat_no_follow(&parent, &name, HostPathKind::Directory)?;
    if identity_from_handle(&directory)? != identity.object_identity() {
        return Err(invalid("quarantine directory handle identity changed"));
    }
    remove_children(&directory)?;
    if fstatat_identity(&parent, &name)? != identity.object_identity() {
        return Err(invalid("quarantine directory changed before final unlink"));
    }
    if unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    parent.sync_all()
}

fn remove_children(directory: &File) -> std::io::Result<()> {
    let duplicate = unsafe { libc::fcntl(directory.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if duplicate < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let stream = unsafe { libc::fdopendir(duplicate) };
    if stream.is_null() {
        let error = std::io::Error::last_os_error();
        unsafe { libc::close(duplicate) };
        return Err(error);
    }
    let result = (|| {
        loop {
            clear_errno();
            let entry = unsafe { libc::readdir(stream) };
            if entry.is_null() {
                let error = current_errno();
                return if error == 0 {
                    Ok(())
                } else {
                    Err(std::io::Error::from_raw_os_error(error))
                };
            }
            let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
            if name.to_bytes() == b"." || name.to_bytes() == b".." {
                continue;
            }
            let before = fstatat(directory, name)?;
            if mode_is_directory(before.st_mode) {
                let child = openat_no_follow(directory, name, HostPathKind::Directory)?;
                let child_identity = identity_from_handle(&child)?;
                if child_identity != identity_from_stat(&before) {
                    return Err(invalid("quarantine child directory identity changed"));
                }
                remove_children(&child)?;
                if fstatat_identity(directory, name)? != child_identity {
                    return Err(invalid("quarantine child directory changed before unlink"));
                }
                if unsafe {
                    libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR)
                } != 0
                {
                    return Err(std::io::Error::last_os_error());
                }
            } else if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
                return Err(std::io::Error::last_os_error());
            }
        }
    })();
    let close_result = unsafe { libc::closedir(stream) };
    if close_result != 0 && result.is_ok() {
        return Err(std::io::Error::last_os_error());
    }
    result
}

fn open_exact_parent(path: &Path, expected: HostFileIdentity) -> std::io::Result<File> {
    let parent_path = path
        .parent()
        .ok_or_else(|| invalid("quarantine path has no parent"))?;
    let parent = open_no_follow(parent_path, HostPathKind::Directory)?;
    if identity_from_handle(&parent)? != expected {
        return Err(invalid("quarantine parent handle identity changed"));
    }
    Ok(parent)
}

fn openat_no_follow(parent: &File, name: &CStr, kind: HostPathKind) -> std::io::Result<File> {
    let mut flags = libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW;
    if kind == HostPathKind::Directory {
        flags |= libc::O_DIRECTORY;
    }
    let descriptor = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let file = unsafe { File::from_raw_fd(descriptor) };
    let metadata = file.metadata()?;
    let matches = match kind {
        HostPathKind::RegularFile => metadata.is_file(),
        HostPathKind::Directory => metadata.is_dir(),
    };
    if !matches {
        return Err(invalid("quarantine entry kind changed"));
    }
    Ok(file)
}

fn fstatat_identity(parent: &File, name: &CStr) -> std::io::Result<HostFileIdentity> {
    Ok(identity_from_stat(&fstatat(parent, name)?))
}

fn fstatat(parent: &File, name: &CStr) -> std::io::Result<libc::stat> {
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { stat.assume_init() })
}

fn identity_from_stat(stat: &libc::stat) -> HostFileIdentity {
    // libc exposes dev_t/ino_t with target-specific integer aliases. The
    // durable project identity is always normalized to u64.
    #[allow(clippy::unnecessary_cast)]
    let device = stat.st_dev as u64;
    #[allow(clippy::unnecessary_cast)]
    let inode = stat.st_ino as u64;
    HostFileIdentity::Unix { device, inode }
}

fn mode_is_directory(mode: libc::mode_t) -> bool {
    mode & libc::S_IFMT == libc::S_IFDIR
}

fn component_cstring(path: &Path) -> std::io::Result<CString> {
    CString::new(
        path.file_name()
            .ok_or_else(|| invalid("quarantine path has no file name"))?
            .as_bytes(),
    )
    .map_err(|_| invalid("quarantine file name contains NUL"))
}

#[cfg(target_os = "linux")]
fn clear_errno() {
    unsafe { *libc::__errno_location() = 0 };
}

#[cfg(target_os = "android")]
fn clear_errno() {
    unsafe { *libc::__errno() = 0 };
}

#[cfg(target_os = "macos")]
fn clear_errno() {
    unsafe { *libc::__error() = 0 };
}

#[cfg(target_os = "linux")]
fn current_errno() -> i32 {
    unsafe { *libc::__errno_location() }
}

#[cfg(target_os = "android")]
fn current_errno() -> i32 {
    unsafe { *libc::__errno() }
}

#[cfg(target_os = "macos")]
fn current_errno() -> i32 {
    unsafe { *libc::__error() }
}
