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
use std::ptr::null;
use std::time::Duration;

use self::attribute_list::ProcessAttributeList;
use self::stdio::ChildStdioHandles;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_FAILED, WAIT_TIMEOUT};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject,
};
use windows_sys::Win32::System::Threading::{
    CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW, EXTENDED_STARTUPINFO_PRESENT,
    GetExitCodeProcess, LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION, ResumeThread,
    STARTF_USESTDHANDLES, STARTUPINFOEXW, STARTUPINFOW, TerminateProcess, WaitForSingleObject,
};

mod attribute_list;
mod stdio;

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
        let stdio = ChildStdioHandles::new(inherit_stdio)
            .map_err(DesktopChildProcessSpawnError::SpawnFailed)?;
        match self.spawn_with_job_list(spec, &stdio) {
            Ok(process) => Ok(process),
            Err(error) if should_fallback_to_suspended_assign(&error) => {
                // Some host jobs reject Job List creation; keep the service suspended until
                // the kill-on-close job owns it so it cannot run as an uncontained backend.
                self.spawn_suspended_then_assign(spec, &stdio)
            }
            Err(error) => Err(DesktopChildProcessSpawnError::SpawnFailed(error)),
        }
    }

    fn spawn_with_job_list(
        &self,
        spec: &NativeProcessSpawnSpec,
        stdio: &ChildStdioHandles,
    ) -> std::io::Result<JobChildProcess> {
        let mut attribute_list = ProcessAttributeList::with_job_and_handles(self.handle, stdio)?;
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
        stdio: &ChildStdioHandles,
    ) -> Result<JobChildProcess, DesktopChildProcessSpawnError> {
        let mut attribute_list = ProcessAttributeList::with_handles(stdio)
            .map_err(DesktopChildProcessSpawnError::SpawnFailed)?;
        let process_info = create_process(
            spec,
            stdio,
            CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT | EXTENDED_STARTUPINFO_PRESENT,
            Some(attribute_list.as_mut_ptr()),
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
    stdio: &ChildStdioHandles,
    creation_flags: u32,
    attribute_list: Option<LPPROC_THREAD_ATTRIBUTE_LIST>,
) -> std::io::Result<PROCESS_INFORMATION> {
    let mut startup: STARTUPINFOEXW = unsafe { zeroed() };
    startup.StartupInfo.cb = if attribute_list.is_some() {
        size_of::<STARTUPINFOEXW>() as u32
    } else {
        size_of::<STARTUPINFOW>() as u32
    };
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = stdio.stdin;
    startup.StartupInfo.hStdOutput = stdio.stdout;
    startup.StartupInfo.hStdError = stdio.stderr;
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
            1,
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

    pub(super) fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> std::io::Result<NativeProcessExitStatus> {
        let timeout_ms = timeout.as_millis().min(u32::MAX as u128) as u32;
        self.wait_for_millis(timeout_ms)
    }

    fn wait_for_millis(&mut self, timeout_ms: u32) -> std::io::Result<NativeProcessExitStatus> {
        if self.process_handle.is_null() {
            return Ok(NativeProcessExitStatus {
                code: None,
                signal: None,
            });
        }
        let wait_result = unsafe { WaitForSingleObject(self.process_handle, timeout_ms) };
        if wait_result == WAIT_FAILED {
            return Err(std::io::Error::last_os_error());
        }
        if wait_result == WAIT_TIMEOUT {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "desktop local service did not exit before stop timeout",
            ));
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
    use windows_sys::Win32::System::Console::{
        GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };

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
                let _ = child.wait_timeout(Duration::from_secs(1));
                panic!("job close did not terminate child");
            }
            sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn non_inherited_stdio_uses_explicit_null_handles() {
        let stdio = ChildStdioHandles::new(false).expect("create null stdio");

        assert!(!stdio.stdin.is_null());
        assert!(!stdio.stdout.is_null());
        assert!(!stdio.stderr.is_null());
        assert_ne!(stdio.stdin, unsafe { GetStdHandle(STD_INPUT_HANDLE) });
        assert_ne!(stdio.stdout, unsafe { GetStdHandle(STD_OUTPUT_HANDLE) });
        assert_ne!(stdio.stderr, unsafe { GetStdHandle(STD_ERROR_HANDLE) });
        assert!(stdio._owned_null_handles.is_some());
    }

    #[test]
    fn process_attribute_list_limits_inheritance_to_stdio_handles() {
        let job = KillOnCloseJob::new().expect("create job");
        let stdio = ChildStdioHandles::new(false).expect("create null stdio");
        let attrs = ProcessAttributeList::with_job_and_handles(job.handle, &stdio)
            .expect("create process attributes");

        assert_eq!(
            attrs.inherited_handles.as_ref(),
            &[stdio.stdin, stdio.stdout, stdio.stderr]
        );
        assert_eq!(
            attrs.jobs.as_ref().expect("job list").as_ref(),
            &[job.handle]
        );
    }
}
