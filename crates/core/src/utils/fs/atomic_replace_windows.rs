//! plan_ref: infra
//!
//! Windows exact-handle atomic replacement adapter.

use std::fs::File;
use std::mem::{offset_of, size_of};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use windows_sys::Win32::Storage::FileSystem::{
    FILE_RENAME_INFO, FileRenameInfo, SetFileInformationByHandle,
};

pub(super) fn replace_file_atomically(
    source_file: &File,
    source: &Path,
    destination: &Path,
) -> std::io::Result<()> {
    super::validate_regular_handle(source_file, source, "atomic replacement temp")?;
    super::ensure_open_file_matches_path(source_file, source, "atomic replacement temp")?;
    let source_parent = source.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "atomic replacement source has no parent",
        )
    })?;
    let destination_parent = destination.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "atomic replacement destination has no parent",
        )
    })?;
    let name = destination.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "atomic replacement destination has no file name",
        )
    })?;
    let source_parent_canonical = std::fs::canonicalize(source_parent)?;
    let destination_parent_canonical = std::fs::canonicalize(destination_parent)?;
    if source_parent_canonical != destination_parent_canonical {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "atomic replacement must remain within one canonical directory",
        ));
    }
    let destination_canonical = destination_parent_canonical.join(name);
    let mut destination_wide = destination_canonical
        .as_os_str()
        .encode_wide()
        .collect::<Vec<_>>();
    let name_bytes = destination_wide
        .len()
        .checked_mul(size_of::<u16>())
        .ok_or_else(|| std::io::Error::other("atomic replacement path is too long"))?;
    let header_bytes = offset_of!(FILE_RENAME_INFO, FileName);
    destination_wide.push(0);
    let buffer_bytes = header_bytes
        .checked_add(name_bytes)
        .and_then(|bytes| bytes.checked_add(size_of::<u16>()))
        .ok_or_else(|| std::io::Error::other("atomic replacement buffer overflow"))?;
    let buffer_words = buffer_bytes.div_ceil(size_of::<usize>());
    let mut buffer = vec![0_usize; buffer_words];
    let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();
    // SAFETY: the aligned buffer owns the fixed header and exact UTF-16 payload.
    unsafe {
        (*info).Anonymous.ReplaceIfExists = true;
        (*info).RootDirectory = std::ptr::null_mut();
        (*info).FileNameLength = u32::try_from(name_bytes).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "atomic replacement path exceeds Win32 length budget",
            )
        })?;
        std::ptr::copy_nonoverlapping(
            destination_wide.as_ptr(),
            (*info).FileName.as_mut_ptr(),
            destination_wide.len(),
        );
    }
    // SAFETY: the handle owns DELETE access and `info` remains valid for the call.
    let renamed = unsafe {
        SetFileInformationByHandle(
            source_file.as_raw_handle(),
            FileRenameInfo,
            info.cast(),
            u32::try_from(buffer_bytes).map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "atomic replacement buffer exceeds Win32 size budget",
                )
            })?,
        )
    };
    if renamed == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
