//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! Windows handle-pinned quarantine mutations.

use super::{HostFileIdentity, HostPathIdentity, HostPathKind, invalid};
use crate::utils::fs::identity::identity_from_handle;
use std::fs::File;
use std::mem::{offset_of, size_of};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use windows_sys::Wdk::Storage::FileSystem::{
    FILE_RENAME_INFORMATION, FileRenameInformation, NtSetInformationFile,
};
use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, RtlNtStatusToDosError};
use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_DISPOSITION_INFO, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, FileDispositionInfo, SetFileInformationByHandle,
};
use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

pub(super) fn native_rename_no_replace(
    original: &HostPathIdentity,
    destination: &Path,
    destination_parent_identity: HostFileIdentity,
) -> std::io::Result<()> {
    let source_parent = open(
        original
            .path()
            .parent()
            .ok_or_else(|| invalid("source parent"))?,
        HostPathKind::Directory,
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
    )?;
    let destination_parent = open(
        destination
            .parent()
            .ok_or_else(|| invalid("destination parent"))?,
        HostPathKind::Directory,
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
    )?;
    let source = open(
        original.path(),
        original.kind(),
        GENERIC_READ | FILE_READ_ATTRIBUTES | DELETE,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
    )?;
    if identity_from_handle(&source_parent)? != original.parent_identity()
        || identity_from_handle(&destination_parent)? != destination_parent_identity
        || identity_from_handle(&source)? != original.object_identity()
    {
        return Err(invalid("quarantine handle identity changed"));
    }
    let destination_name = destination
        .file_name()
        .ok_or_else(|| invalid("quarantine destination has no file name"))?;
    let name = destination_name.encode_wide().collect::<Vec<_>>();
    let name_bytes = name
        .len()
        .checked_mul(size_of::<u16>())
        .ok_or_else(|| invalid("quarantine destination name is too long"))?;
    let header_bytes = offset_of!(FILE_RENAME_INFORMATION, FileName);
    let buffer_bytes = header_bytes
        .checked_add(name_bytes)
        .ok_or_else(|| invalid("quarantine rename buffer overflow"))?;
    let mut buffer = vec![0_usize; buffer_bytes.div_ceil(size_of::<usize>())];
    let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
    unsafe {
        (*info).Anonymous.ReplaceIfExists = false;
        // The leaf name is resolved relative to the exact, already-validated
        // destination-parent handle. No absolute pathname is reparsed here.
        (*info).RootDirectory = destination_parent.as_raw_handle();
        (*info).FileNameLength = u32::try_from(name_bytes)
            .map_err(|_| invalid("quarantine destination exceeds Win32 length budget"))?;
        std::ptr::copy_nonoverlapping(name.as_ptr(), (*info).FileName.as_mut_ptr(), name.len());
    }
    let mut status_block = IO_STATUS_BLOCK::default();
    let status = unsafe {
        NtSetInformationFile(
            source.as_raw_handle(),
            &mut status_block,
            info.cast(),
            u32::try_from(buffer_bytes)
                .map_err(|_| invalid("quarantine rename buffer exceeds Win32 budget"))?,
            FileRenameInformation,
        )
    };
    if status < 0 {
        let code = unsafe { RtlNtStatusToDosError(status) };
        Err(std::io::Error::from_raw_os_error(code as i32))
    } else {
        sync_directory_handle(&source_parent)?;
        if source_parent.as_raw_handle() != destination_parent.as_raw_handle() {
            sync_directory_handle(&destination_parent)?;
        }
        Ok(())
    }
}

pub(super) fn delete_file_pinned(identity: &HostPathIdentity) -> std::io::Result<()> {
    let parent = open(
        identity
            .path()
            .parent()
            .ok_or_else(|| invalid("file parent"))?,
        HostPathKind::Directory,
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
    )?;
    let file = open(
        identity.path(),
        HostPathKind::RegularFile,
        GENERIC_READ | DELETE,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
    )?;
    if identity_from_handle(&parent)? != identity.parent_identity()
        || identity_from_handle(&file)? != identity.object_identity()
    {
        return Err(invalid("quarantine file handle identity changed"));
    }
    mark_delete(&file)?;
    drop(file);
    sync_directory_handle(&parent)
}

pub(super) fn delete_directory_pinned(identity: &HostPathIdentity) -> std::io::Result<()> {
    let parent = open(
        identity
            .path()
            .parent()
            .ok_or_else(|| invalid("directory parent"))?,
        HostPathKind::Directory,
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
    )?;
    // Omitting FILE_SHARE_DELETE pins this exact root pathname while the
    // standard library's handle-relative, no-follow child walker runs.
    let directory = open(
        identity.path(),
        HostPathKind::Directory,
        FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | DELETE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
    )?;
    if identity_from_handle(&parent)? != identity.parent_identity()
        || identity_from_handle(&directory)? != identity.object_identity()
    {
        return Err(invalid("quarantine directory handle identity changed"));
    }
    let entries = std::fs::read_dir(identity.path())?
        .map(|entry| entry.map(|value| value.path()))
        .collect::<Result<Vec<_>, _>>()?;
    for entry in entries {
        let metadata = std::fs::symlink_metadata(&entry)?;
        use std::os::windows::fs::MetadataExt;
        let is_reparse = metadata.file_attributes() & 0x0400 != 0;
        if metadata.is_dir() && !is_reparse {
            std::fs::remove_dir_all(&entry)?;
        } else if metadata.is_dir() {
            std::fs::remove_dir(&entry)?;
        } else {
            std::fs::remove_file(&entry)?;
        }
    }
    mark_delete(&directory)?;
    drop(directory);
    sync_directory_handle(&parent)
}

fn sync_directory_handle(directory: &File) -> std::io::Result<()> {
    directory.sync_all().map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!("failed to flush exact quarantine parent handle: {error}"),
        )
    })
}

fn mark_delete(file: &File) -> std::io::Result<()> {
    let mut disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    let deleted = unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle(),
            FileDispositionInfo,
            (&mut disposition as *mut FILE_DISPOSITION_INFO).cast(),
            u32::try_from(size_of::<FILE_DISPOSITION_INFO>())
                .map_err(|_| invalid("invalid file disposition size"))?,
        )
    };
    if deleted == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn open(path: &Path, kind: HostPathKind, access: u32, share: u32) -> std::io::Result<File> {
    let file = std::fs::OpenOptions::new()
        .access_mode(access)
        .share_mode(share)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let metadata = file.metadata()?;
    use std::os::windows::fs::MetadataExt;
    if metadata.file_attributes() & 0x0400 != 0
        || (kind == HostPathKind::Directory && !metadata.is_dir())
        || (kind == HostPathKind::RegularFile && !metadata.is_file())
    {
        return Err(invalid(
            "quarantine handle kind or reparse identity changed",
        ));
    }
    Ok(file)
}
