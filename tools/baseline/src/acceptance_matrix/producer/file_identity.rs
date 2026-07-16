//! Cross-platform filesystem object identity for pinned artifact roots.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

#[cfg(unix)]
use anyhow::Context;
use anyhow::Result;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct FileIdentity {
    storage: u64,
    file: u64,
}

impl FileIdentity {
    #[cfg(unix)]
    pub(super) fn read(path: &Path) -> Result<Self> {
        use std::os::unix::fs::MetadataExt;
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("failed to inspect artifact root {}", path.display()))?;
        Ok(Self {
            storage: metadata.dev(),
            file: metadata.ino(),
        })
    }

    #[cfg(windows)]
    pub(super) fn read(path: &Path) -> Result<Self> {
        windows::read(path)
    }

    #[cfg(not(any(unix, windows)))]
    pub(super) fn read(path: &Path) -> Result<Self> {
        anyhow::bail!(
            "artifact root identity is unsupported on this platform: {}",
            path.display()
        )
    }
}

#[cfg(windows)]
mod windows {
    use super::FileIdentity;
    use anyhow::{Context, Result, bail};
    use std::ffi::{OsStr, c_void};
    use std::mem::MaybeUninit;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::ptr;

    type RawHandle = *mut c_void;
    const INVALID_HANDLE_VALUE: RawHandle = -1isize as RawHandle;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const FILE_SHARE_DELETE: u32 = 0x0000_0004;
    const OPEN_EXISTING: u32 = 3;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct FileTime {
        dwLowDateTime: u32,
        dwHighDateTime: u32,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct ByHandleFileInformation {
        dwFileAttributes: u32,
        ftCreationTime: FileTime,
        ftLastAccessTime: FileTime,
        ftLastWriteTime: FileTime,
        dwVolumeSerialNumber: u32,
        nFileSizeHigh: u32,
        nFileSizeLow: u32,
        nNumberOfLinks: u32,
        nFileIndexHigh: u32,
        nFileIndexLow: u32,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateFileW(
            file_name: *const u16,
            desired_access: u32,
            share_mode: u32,
            security_attributes: *const c_void,
            creation_disposition: u32,
            flags_and_attributes: u32,
            template_file: RawHandle,
        ) -> RawHandle;
        fn GetFileInformationByHandle(
            file: RawHandle,
            information: *mut ByHandleFileInformation,
        ) -> i32;
        fn CloseHandle(object: RawHandle) -> i32;
    }

    struct OwnedHandle(RawHandle);

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            // SAFETY: this wrapper is constructed only from a valid, uniquely
            // owned CreateFileW handle and closes it exactly once.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    pub(super) fn read(path: &Path) -> Result<FileIdentity> {
        let wide = OsStr::new(path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        // SAFETY: `wide` is NUL-terminated and remains alive for the call;
        // all optional pointers are null and the returned handle is checked.
        let raw = unsafe {
            CreateFileW(
                wide.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                ptr::null_mut(),
            )
        };
        if raw == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error())
                .with_context(|| format!("failed to open artifact root {}", path.display()));
        }
        let handle = OwnedHandle(raw);
        let mut information = MaybeUninit::<ByHandleFileInformation>::uninit();
        // SAFETY: `handle` is valid and `information` points to writable,
        // correctly sized storage initialized by the Windows API on success.
        let status = unsafe { GetFileInformationByHandle(handle.0, information.as_mut_ptr()) };
        if status == 0 {
            bail!(
                "failed to identify artifact root {}: {}",
                path.display(),
                std::io::Error::last_os_error()
            );
        }
        // SAFETY: the API returned success and initialized the full structure.
        let information = unsafe { information.assume_init() };
        Ok(FileIdentity {
            storage: u64::from(information.dwVolumeSerialNumber),
            file: (u64::from(information.nFileIndexHigh) << 32)
                | u64::from(information.nFileIndexLow),
        })
    }
}
