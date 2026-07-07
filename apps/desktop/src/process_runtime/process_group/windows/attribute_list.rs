//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-process-adapter-decision

use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::System::Threading::{
    DeleteProcThreadAttributeList, InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
    PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROC_THREAD_ATTRIBUTE_JOB_LIST, UpdateProcThreadAttribute,
};

use super::stdio::ChildStdioHandles;

pub(super) struct ProcessAttributeList {
    buffer: Vec<u8>,
    initialized: bool,
    pub(super) jobs: Option<Box<[HANDLE; 1]>>,
    pub(super) inherited_handles: Box<[HANDLE]>,
}

impl ProcessAttributeList {
    pub(super) fn with_job_and_handles(
        job_handle: HANDLE,
        stdio: &ChildStdioHandles,
    ) -> std::io::Result<Self> {
        Self::new(Some(job_handle), stdio)
    }

    pub(super) fn with_handles(stdio: &ChildStdioHandles) -> std::io::Result<Self> {
        Self::new(None, stdio)
    }

    fn new(job_handle: Option<HANDLE>, stdio: &ChildStdioHandles) -> std::io::Result<Self> {
        let attribute_count = if job_handle.is_some() { 2 } else { 1 };
        let mut size = 0usize;
        unsafe {
            InitializeProcThreadAttributeList(null_mut(), attribute_count, 0, &mut size);
        }
        if size == 0 {
            return Err(std::io::Error::last_os_error());
        }

        let mut attribute_list = Self {
            buffer: vec![0; size],
            initialized: false,
            jobs: job_handle.map(|handle| Box::new([handle])),
            inherited_handles: stdio.inherited_handle_list().into_boxed_slice(),
        };
        let initialized = unsafe {
            InitializeProcThreadAttributeList(
                attribute_list.as_mut_ptr(),
                attribute_count,
                0,
                &mut size,
            )
        };
        if initialized == 0 {
            return Err(std::io::Error::last_os_error());
        }
        attribute_list.initialized = true;

        attribute_list.update_handle_list()?;
        if attribute_list.jobs.is_some() {
            attribute_list.update_job_list()?;
        }

        Ok(attribute_list)
    }

    fn update_job_list(&mut self) -> std::io::Result<()> {
        let Some(jobs_ptr) = self
            .jobs
            .as_ref()
            .map(|jobs| jobs.as_ptr() as *const c_void)
        else {
            return Ok(());
        };
        let updated = unsafe {
            UpdateProcThreadAttribute(
                self.as_mut_ptr(),
                0,
                PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
                jobs_ptr,
                size_of::<[HANDLE; 1]>(),
                null_mut(),
                null(),
            )
        };
        if updated == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    fn update_handle_list(&mut self) -> std::io::Result<()> {
        let handles_ptr = self.inherited_handles.as_ptr() as *const c_void;
        let handles_size = size_of::<HANDLE>() * self.inherited_handles.len();
        let updated = unsafe {
            UpdateProcThreadAttribute(
                self.as_mut_ptr(),
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                handles_ptr,
                handles_size,
                null_mut(),
                null(),
            )
        };
        if updated == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    pub(super) fn as_mut_ptr(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.buffer.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST
    }
}

impl Drop for ProcessAttributeList {
    fn drop(&mut self) {
        if self.initialized {
            unsafe {
                DeleteProcThreadAttributeList(self.as_mut_ptr());
            }
        }
    }
}
