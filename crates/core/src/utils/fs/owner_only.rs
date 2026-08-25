//! plan_ref:
//!   - 08_auth#key-and-file-permissions
//!
//! Windows owner-only DACL construction and exact-handle enforcement.

use std::fs::{File, OpenOptions};
use std::io;
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::path::Path;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_INSUFFICIENT_BUFFER, GENERIC_READ, GENERIC_WRITE, HANDLE,
    INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Security::Authorization::{SE_FILE_OBJECT, SetSecurityInfo};
use windows_sys::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACL, ACL_REVISION, AddAccessAllowedAce, CopySid, DACL_SECURITY_INFORMATION,
    GetLengthSid, GetTokenInformation, InitializeAcl, InitializeSecurityDescriptor,
    PROTECTED_DACL_SECURITY_INFORMATION, PSID, SE_DACL_PROTECTED, SECURITY_ATTRIBUTES,
    SECURITY_DESCRIPTOR, SetSecurityDescriptorControl, SetSecurityDescriptorDacl, TOKEN_QUERY,
    TOKEN_USER, TokenUser,
};
use windows_sys::Win32::Storage::FileSystem::{
    CREATE_NEW, CreateFileW, FILE_ALL_ACCESS, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, READ_CONTROL, WRITE_DAC,
};
use windows_sys::Win32::System::SystemServices::SECURITY_DESCRIPTOR_REVISION;
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

pub(super) fn create_owner_only_regular_file_new(path: &Path, context: &str) -> io::Result<File> {
    with_owner_acl(|acl| {
        let mut descriptor: SECURITY_DESCRIPTOR = unsafe { zeroed() };
        if unsafe {
            InitializeSecurityDescriptor(
                (&mut descriptor as *mut SECURITY_DESCRIPTOR).cast(),
                SECURITY_DESCRIPTOR_REVISION,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if unsafe {
            SetSecurityDescriptorDacl(
                (&mut descriptor as *mut SECURITY_DESCRIPTOR).cast(),
                1,
                acl,
                0,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if unsafe {
            SetSecurityDescriptorControl(
                (&mut descriptor as *mut SECURITY_DESCRIPTOR).cast(),
                SE_DACL_PROTECTED,
                SE_DACL_PROTECTED,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: (&mut descriptor as *mut SECURITY_DESCRIPTOR).cast(),
            bInheritHandle: 0,
        };
        let mut path_wide = path.as_os_str().encode_wide().collect::<Vec<_>>();
        path_wide.push(0);
        let handle = unsafe {
            CreateFileW(
                path_wide.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                &attributes,
                CREATE_NEW,
                FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            let error = io::Error::last_os_error();
            return Err(io::Error::new(
                error.kind(),
                format!("Failed to create {context} with an owner-only DACL at {path:?}: {error}"),
            ));
        }
        Ok(unsafe { File::from_raw_handle(handle) })
    })
}

pub(super) fn open_owner_only_regular_file_read(path: &Path, context: &str) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options
        .access_mode(GENERIC_READ | READ_CONTROL | WRITE_DAC)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    let file = options.open(path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("Failed to open {context} for owner-only enforcement at {path:?}: {error}"),
        )
    })?;
    with_owner_acl(|acl| {
        let result = unsafe {
            SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                acl,
                null(),
            )
        };
        if result != 0 {
            return Err(io::Error::from_raw_os_error(result as i32));
        }
        Ok(())
    })?;
    Ok(file)
}

fn with_owner_acl<T>(action: impl FnOnce(*mut ACL) -> io::Result<T>) -> io::Result<T> {
    let sid = current_user_sid()?;
    let acl_bytes = size_of::<ACL>()
        .checked_add(size_of::<ACCESS_ALLOWED_ACE>())
        .and_then(|bytes| bytes.checked_sub(size_of::<u32>()))
        .and_then(|bytes| bytes.checked_add(sid.len()))
        .ok_or_else(|| io::Error::other("owner-only ACL size overflow"))?;
    let mut acl_storage = vec![0usize; acl_bytes.div_ceil(size_of::<usize>())];
    let acl = acl_storage.as_mut_ptr().cast::<ACL>();
    if unsafe { InitializeAcl(acl, acl_bytes as u32, ACL_REVISION) } == 0 {
        return Err(io::Error::last_os_error());
    }
    if unsafe {
        AddAccessAllowedAce(
            acl,
            ACL_REVISION,
            FILE_ALL_ACCESS,
            sid.as_ptr().cast_mut().cast(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    action(acl)
}

fn current_user_sid() -> io::Result<Vec<u8>> {
    let mut token: HANDLE = null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = TokenHandle(token);
    let mut required = 0u32;
    let first = unsafe { GetTokenInformation(token.0, TokenUser, null_mut(), 0, &mut required) };
    if first != 0
        || io::Error::last_os_error().raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER as i32)
    {
        return Err(io::Error::last_os_error());
    }
    let mut token_info = vec![0usize; (required as usize).div_ceil(size_of::<usize>())];
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            token_info.as_mut_ptr().cast(),
            required,
            &mut required,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let token_user = unsafe { &*token_info.as_ptr().cast::<TOKEN_USER>() };
    let sid_len = unsafe { GetLengthSid(token_user.User.Sid) };
    if sid_len == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut sid = vec![0u8; sid_len as usize];
    if unsafe {
        CopySid(
            sid_len,
            sid.as_mut_ptr().cast::<core::ffi::c_void>() as PSID,
            token_user.User.Sid,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(sid)
}

struct TokenHandle(HANDLE);

impl Drop for TokenHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::GetSecurityInfo;
    use windows_sys::Win32::Security::{
        ACL_SIZE_INFORMATION, AclSizeInformation, EqualSid, GetAce, GetAclInformation,
        GetSecurityDescriptorControl, PSECURITY_DESCRIPTOR,
    };
    use windows_sys::Win32::System::SystemServices::ACCESS_ALLOWED_ACE_TYPE;

    #[test]
    fn created_owner_only_file_has_one_protected_dacl_entry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("owner-only.key");
        let file = create_owner_only_regular_file_new(&path, "owner-only test")
            .expect("owner-only create");
        let mut dacl: *mut ACL = null_mut();
        let mut descriptor: PSECURITY_DESCRIPTOR = null_mut();
        let result = unsafe {
            GetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                &mut dacl,
                null_mut(),
                &mut descriptor,
            )
        };
        assert_eq!(result, 0, "GetSecurityInfo failed: {result}");
        assert!(!descriptor.is_null());
        assert!(!dacl.is_null());

        let mut info: ACL_SIZE_INFORMATION = unsafe { zeroed() };
        assert_ne!(
            unsafe {
                GetAclInformation(
                    dacl,
                    (&mut info as *mut ACL_SIZE_INFORMATION).cast(),
                    size_of::<ACL_SIZE_INFORMATION>() as u32,
                    AclSizeInformation,
                )
            },
            0
        );
        let mut control = 0u16;
        let mut revision = 0u32;
        assert_ne!(
            unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) },
            0
        );
        assert_eq!(info.AceCount, 1);
        assert_ne!(control & SE_DACL_PROTECTED, 0);

        let mut ace: *mut core::ffi::c_void = null_mut();
        assert_ne!(unsafe { GetAce(dacl, 0, &mut ace) }, 0);
        assert!(!ace.is_null());
        let ace = unsafe { &*ace.cast::<ACCESS_ALLOWED_ACE>() };
        assert_eq!(u32::from(ace.Header.AceType), ACCESS_ALLOWED_ACE_TYPE);
        assert_eq!(ace.Mask, FILE_ALL_ACCESS);
        let owner_sid = current_user_sid().expect("current user SID");
        let ace_sid = (&ace.SidStart as *const u32).cast_mut().cast();
        assert_ne!(
            unsafe { EqualSid(ace_sid, owner_sid.as_ptr().cast_mut().cast()) },
            0,
            "the only DACL entry must belong to the current user"
        );

        unsafe {
            LocalFree(descriptor);
        }
    }
}
