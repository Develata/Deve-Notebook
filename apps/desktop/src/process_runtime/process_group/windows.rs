//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-process-adapter-decision

use super::DesktopChildProcessSpawnError;
use deve_core::native_adapter::{
    NativeProcessEnvBinding, NativeProcessExitStatus, NativeProcessSpawnSpec,
};
use std::ffi::{OsStr, c_void};
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_FAILED};
use windows_sys::Win32::System::Console::{
    GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject,
};
use windows_sys::Win32::System::Threading::{
    CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW, DeleteProcThreadAttributeList,
    EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess, INFINITE, InitializeProcThreadAttributeList,
    LPPROC_THREAD_ATTRIBUTE_LIST, PROC_THREAD_ATTRIBUTE_JOB_LIST, PROCESS_INFORMATION,
    ResumeThread, STARTF_USESTDHANDLES, STARTUPINFOEXW, STARTUPINFOW, TerminateProcess,
    UpdateProcThreadAttribute, WaitForSingleObject,
};

#[derive(Debug)]
pub(super) struct KillOnCloseJob {
    handle: HANDLE,
}

#[derive(Debug)]
pub(super) struct JobChildProcess {
    process_handle: HANDLE,
    pid: u32,
}

impl KillOnCloseJob {
    pub(super) fn new() -> std::io::Result<Self> {
        let handle = unsafe { CreateJobObjectW(null(), null()) };
        if handle.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        let job = Self { handle };
        if let Err(error) = job.enable_kill_on_close() {
            drop(job);
            return Err(error);
        }
        Ok(job)
    }

    pub(super) fn spawn_service(
        &self,
        spec: &NativeProcessSpawnSpec,
        inherit_stdio: bool,
    ) -> Result<JobChildProcess, DesktopChildProcessSpawnError> {
        let stdio = ChildStdioHandles::new(inherit_stdio);
        match self.spawn_with_job_list(spec, stdio.as_ref()) {
            Ok(process) => Ok(process),
            Err(error) if should_fallback_to_suspended_assign(&error) => {
                // Some host jobs reject Job List creation; keep the service suspended until
                // the kill-on-close job owns it so it cannot run as an uncontained backend.
                self.spawn_suspended_then_assign(spec, stdio.as_ref())
            }
            Err(error) => Err(DesktopChildProcessSpawnError::SpawnFailed(error)),
        }
    }

    fn spawn_with_job_list(
        &self,
        spec: &NativeProcessSpawnSpec,
        stdio: Option<&ChildStdioHandles>,
    ) -> std::io::Result<JobChildProcess> {
        let mut attribute_list = JobAttributeList::new(self.handle)?;
        let process_info = create_process(
            spec,
            stdio,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT,
            Some(attribute_list.as_mut_ptr()),
        )?;
        Ok(process_from_info(process_info))
    }

    fn spawn_suspended_then_assign(
        &self,
        spec: &NativeProcessSpawnSpec,
        stdio: Option<&ChildStdioHandles>,
    ) -> Result<JobChildProcess, DesktopChildProcessSpawnError> {
        let process_info = create_process(
            spec,
            stdio,
            CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
            None,
        )
        .map_err(DesktopChildProcessSpawnError::SpawnFailed)?;

        let assigned = unsafe { AssignProcessToJobObject(self.handle, process_info.hProcess) };
        if assigned == 0 {
            let error = std::io::Error::last_os_error();
            terminate_and_close(process_info);
            return Err(DesktopChildProcessSpawnError::ContainmentFailed(error));
        }

        let previous_suspend_count = unsafe { ResumeThread(process_info.hThread) };
        if previous_suspend_count == u32::MAX {
            let error = std::io::Error::last_os_error();
            terminate_and_close(process_info);
            return Err(DesktopChildProcessSpawnError::ContainmentFailed(error));
        }

        Ok(process_from_info(process_info))
    }

    fn enable_kill_on_close(&self) -> std::io::Result<()> {
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let configured = unsafe {
            SetInformationJobObject(
                self.handle,
                JobObjectExtendedLimitInformation,
                &limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION as *const c_void,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if configured == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
}

fn create_process(
    spec: &NativeProcessSpawnSpec,
    stdio: Option<&ChildStdioHandles>,
    creation_flags: u32,
    attribute_list: Option<LPPROC_THREAD_ATTRIBUTE_LIST>,
) -> std::io::Result<PROCESS_INFORMATION> {
    let mut startup: STARTUPINFOEXW = unsafe { zeroed() };
    startup.StartupInfo.cb = if attribute_list.is_some() {
        size_of::<STARTUPINFOEXW>() as u32
    } else {
        size_of::<STARTUPINFOW>() as u32
    };
    if let Some(stdio) = stdio {
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = stdio.stdin;
        startup.StartupInfo.hStdOutput = stdio.stdout;
        startup.StartupInfo.hStdError = stdio.stderr;
    }
    if let Some(attribute_list) = attribute_list {
        startup.lpAttributeList = attribute_list;
    }

    let application_name = wide_path_null(&spec.executable);
    let mut command_line = command_line(&spec.executable, &spec.argv);
    let current_dir = wide_path_null(&spec.cwd);
    let environment = environment_block(&spec.env);
    let mut process_info: PROCESS_INFORMATION = unsafe { zeroed() };

    let created = unsafe {
        CreateProcessW(
            application_name.as_ptr(),
            command_line.as_mut_ptr(),
            null(),
            null(),
            if stdio.is_some() { 1 } else { 0 },
            creation_flags,
            environment.as_ptr() as *const c_void,
            current_dir.as_ptr(),
            &startup as *const STARTUPINFOEXW as *const _,
            &mut process_info,
        )
    };
    if created == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(process_info)
}

fn process_from_info(process_info: PROCESS_INFORMATION) -> JobChildProcess {
    unsafe {
        CloseHandle(process_info.hThread);
    }
    JobChildProcess {
        process_handle: process_info.hProcess,
        pid: process_info.dwProcessId,
    }
}

fn terminate_and_close(process_info: PROCESS_INFORMATION) {
    unsafe {
        TerminateProcess(process_info.hProcess, 1);
        CloseHandle(process_info.hThread);
        CloseHandle(process_info.hProcess);
    }
}

fn should_fallback_to_suspended_assign(error: &std::io::Error) -> bool {
    matches!(error.raw_os_error(), Some(5 | 87))
}

impl Drop for KillOnCloseJob {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                CloseHandle(self.handle);
            }
            self.handle = null::<c_void>() as HANDLE;
        }
    }
}

unsafe impl Send for KillOnCloseJob {}

impl JobChildProcess {
    pub(super) fn id(&self) -> u32 {
        self.pid
    }

    pub(super) fn kill(&mut self) -> std::io::Result<()> {
        if self.process_handle.is_null() {
            return Ok(());
        }
        let terminated = unsafe { TerminateProcess(self.process_handle, 1) };
        if terminated == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    pub(super) fn wait(&mut self) -> std::io::Result<NativeProcessExitStatus> {
        if self.process_handle.is_null() {
            return Ok(NativeProcessExitStatus {
                code: None,
                signal: None,
            });
        }
        let wait_result = unsafe { WaitForSingleObject(self.process_handle, INFINITE) };
        if wait_result == WAIT_FAILED {
            return Err(std::io::Error::last_os_error());
        }
        let mut exit_code = 0u32;
        let got_exit_code = unsafe { GetExitCodeProcess(self.process_handle, &mut exit_code) };
        if got_exit_code == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(NativeProcessExitStatus {
            code: Some(exit_code as i32),
            signal: None,
        })
    }
}

impl Drop for JobChildProcess {
    fn drop(&mut self) {
        if !self.process_handle.is_null() {
            unsafe {
                CloseHandle(self.process_handle);
            }
            self.process_handle = null::<c_void>() as HANDLE;
        }
    }
}

unsafe impl Send for JobChildProcess {}

struct JobAttributeList {
    buffer: Vec<u8>,
    jobs: Box<[HANDLE; 1]>,
}

impl JobAttributeList {
    fn new(job_handle: HANDLE) -> std::io::Result<Self> {
        let mut size = 0usize;
        unsafe {
            InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut size);
        }
        if size == 0 {
            return Err(std::io::Error::last_os_error());
        }

        let mut attribute_list = Self {
            buffer: vec![0; size],
            jobs: Box::new([job_handle]),
        };
        let initialized = unsafe {
            InitializeProcThreadAttributeList(attribute_list.as_mut_ptr(), 1, 0, &mut size)
        };
        if initialized == 0 {
            return Err(std::io::Error::last_os_error());
        }

        let updated = unsafe {
            UpdateProcThreadAttribute(
                attribute_list.as_mut_ptr(),
                0,
                PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
                attribute_list.jobs.as_ptr() as *const c_void,
                size_of::<[HANDLE; 1]>(),
                null_mut(),
                null(),
            )
        };
        if updated == 0 {
            return Err(std::io::Error::last_os_error());
        }

        Ok(attribute_list)
    }

    fn as_mut_ptr(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.buffer.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST
    }
}

impl Drop for JobAttributeList {
    fn drop(&mut self) {
        unsafe {
            DeleteProcThreadAttributeList(self.as_mut_ptr());
        }
    }
}

struct ChildStdioHandles {
    stdin: HANDLE,
    stdout: HANDLE,
    stderr: HANDLE,
}

impl ChildStdioHandles {
    fn new(inherit_stdio: bool) -> Option<Self> {
        if inherit_stdio {
            Some(Self {
                stdin: unsafe { GetStdHandle(STD_INPUT_HANDLE) },
                stdout: unsafe { GetStdHandle(STD_OUTPUT_HANDLE) },
                stderr: unsafe { GetStdHandle(STD_ERROR_HANDLE) },
            })
        } else {
            None
        }
    }
}

fn wide_path_null(path: &Path) -> Vec<u16> {
    let mut value: Vec<u16> = path.as_os_str().encode_wide().collect();
    value.push(0);
    value
}

fn environment_block(env: &[NativeProcessEnvBinding]) -> Vec<u16> {
    let mut entries: Vec<String> = env
        .iter()
        .map(|binding| format!("{}={}", binding.key, binding.value))
        .collect();
    entries.sort_by_key(|entry| entry.to_ascii_uppercase());

    let mut block = Vec::new();
    for entry in entries {
        block.extend(entry.encode_utf16());
        block.push(0);
    }
    block.push(0);
    block
}

fn command_line(executable: &Path, argv: &[String]) -> Vec<u16> {
    let mut command = Vec::new();
    push_windows_arg(&mut command, executable.as_os_str());
    for arg in argv {
        command.push(' ' as u16);
        push_windows_arg(&mut command, OsStr::new(arg));
    }
    command.push(0);
    command
}

fn push_windows_arg(command: &mut Vec<u16>, arg: &OsStr) {
    let arg: Vec<u16> = arg.encode_wide().collect();
    let needs_quotes = arg.is_empty()
        || arg.iter().any(
            |ch| matches!(*ch, value if value == ' ' as u16 || value == '\t' as u16 || value == '"' as u16),
        );
    if !needs_quotes {
        command.extend(arg);
        return;
    }

    command.push('"' as u16);
    let mut backslashes = 0usize;
    for ch in arg {
        if ch == '\\' as u16 {
            backslashes += 1;
            continue;
        }
        if ch == '"' as u16 {
            command.extend(std::iter::repeat_n('\\' as u16, backslashes * 2 + 1));
            command.push(ch);
            backslashes = 0;
            continue;
        }
        command.extend(std::iter::repeat_n('\\' as u16, backslashes));
        backslashes = 0;
        command.push(ch);
    }
    command.extend(std::iter::repeat_n('\\' as u16, backslashes * 2));
    command.push('"' as u16);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::{Duration, Instant};

    #[test]
    fn kill_on_close_job_terminates_assigned_child() {
        let job = KillOnCloseJob::new().expect("create job");
        let spec = crate::process_runtime_test::support::windows_cmd_ping_spawn_spec();
        let mut child = job
            .spawn_service(&spec, false)
            .expect("spawn long-lived child in job");

        drop(job);

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let mut exit_code = 0u32;
            let got_exit_code = unsafe { GetExitCodeProcess(child.process_handle, &mut exit_code) };
            assert_ne!(got_exit_code, 0, "poll child exit code");
            if exit_code != windows_sys::Win32::Foundation::STILL_ACTIVE as u32 {
                return;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("job close did not terminate child");
            }
            sleep(Duration::from_millis(50));
        }
    }
}
