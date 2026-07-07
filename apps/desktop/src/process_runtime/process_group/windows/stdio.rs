//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-process-adapter-decision

use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{
    HANDLE, HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE, SetHandleInformation,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Console::{
    GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};

pub(super) struct ChildStdioHandles {
    pub(super) stdin: HANDLE,
    pub(super) stdout: HANDLE,
    pub(super) stderr: HANDLE,
    pub(super) _owned_null_handles: Option<ChildNullStdioHandles>,
}

pub(super) struct ChildNullStdioHandles {
    _stdin: OwnedHandle,
    _stdout: OwnedHandle,
    _stderr: OwnedHandle,
}

impl ChildStdioHandles {
    pub(super) fn new(inherit_stdio: bool) -> std::io::Result<Self> {
        if inherit_stdio {
            return Ok(Self {
                stdin: unsafe { GetStdHandle(STD_INPUT_HANDLE) },
                stdout: unsafe { GetStdHandle(STD_OUTPUT_HANDLE) },
                stderr: unsafe { GetStdHandle(STD_ERROR_HANDLE) },
                _owned_null_handles: None,
            });
        }

        let stdin = open_inheritable_nul(FILE_GENERIC_READ)?;
        let stdout = open_inheritable_nul(FILE_GENERIC_WRITE)?;
        let stderr = open_inheritable_nul(FILE_GENERIC_WRITE)?;
        Ok(Self {
            stdin: stdin.as_raw_handle() as HANDLE,
            stdout: stdout.as_raw_handle() as HANDLE,
            stderr: stderr.as_raw_handle() as HANDLE,
            _owned_null_handles: Some(ChildNullStdioHandles {
                _stdin: stdin,
                _stdout: stdout,
                _stderr: stderr,
            }),
        })
    }

    pub(super) fn inherited_handle_list(&self) -> Vec<HANDLE> {
        vec![self.stdin, self.stdout, self.stderr]
    }
}

fn open_inheritable_nul(access: u32) -> std::io::Result<OwnedHandle> {
    let nul = wide_null_terminated("NUL");
    let handle = unsafe {
        CreateFileW(
            nul.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error());
    }
    let owned = unsafe { OwnedHandle::from_raw_handle(handle as _) };
    mark_inheritable(&owned)?;
    Ok(owned)
}

fn mark_inheritable(handle: &OwnedHandle) -> std::io::Result<()> {
    let updated = unsafe {
        SetHandleInformation(
            handle.as_raw_handle() as HANDLE,
            HANDLE_FLAG_INHERIT,
            HANDLE_FLAG_INHERIT,
        )
    };
    if updated == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn wide_null_terminated(value: &str) -> Vec<u16> {
    let mut encoded: Vec<u16> = value.encode_utf16().collect();
    encoded.push(0);
    encoded
}
