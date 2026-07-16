//! Bounded producer process and process-tree lifecycle.
//! plan_ref: 18_release#first-tag-acceptance-matrix

use super::model::CommandStep;
use anyhow::{Context, Result, bail};
#[cfg(any(unix, windows))]
use std::io;
#[cfg(unix)]
use std::num::TryFromIntError;
use std::path::Path;
use std::process::{Child, Command, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(100);
const TERMINATION_WAIT: Duration = Duration::from_secs(5);

pub(in crate::acceptance_matrix) fn run_step(
    root: &Path,
    step: &CommandStep,
    timeout: Duration,
) -> Result<ExitStatus> {
    let mut command = Command::new(&step.program);
    command.args(&step.args).envs(&step.env).current_dir(root);
    configure_process_group(&mut command);
    let mut child = command
        .spawn()
        .with_context(|| format!("failed to start command step {}", step.program))?;
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if started.elapsed() >= timeout {
            let termination = terminate_owned_processes(&mut child);
            if !wait_for_exit(&mut child, TERMINATION_WAIT)? {
                bail!(
                    "command step {} exceeded {} seconds and its direct child did not terminate within {} seconds",
                    step.program,
                    timeout.as_secs(),
                    TERMINATION_WAIT.as_secs()
                );
            }
            if let Err(error) = termination {
                bail!(
                    "command step {} exceeded {} seconds; owned-process cleanup failed closed: {error}",
                    step.program,
                    timeout.as_secs()
                );
            }
            bail!(
                "command step {} exceeded {} seconds",
                step.program,
                timeout.as_secs()
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_exit(child: &mut Child, timeout: Duration) -> Result<bool> {
    let started = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            return Ok(true);
        }
        if started.elapsed() >= timeout {
            return Ok(false);
        }
        thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(windows)]
fn configure_process_group(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    command.creation_flags(CREATE_NEW_PROCESS_GROUP);
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(not(any(unix, windows)))]
fn configure_process_group(_command: &mut Command) {}

#[cfg(windows)]
fn terminate_owned_processes(child: &mut Child) -> Result<()> {
    let pid = child.id();
    let tree_result = Command::new("taskkill")
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .status()
        .map(|status| status.success());
    let direct_result = child.kill();
    validate_windows_tree_termination(pid, tree_result, direct_result)
}

#[cfg(windows)]
fn validate_windows_tree_termination(
    pid: u32,
    tree_result: io::Result<bool>,
    direct_result: io::Result<()>,
) -> Result<()> {
    match tree_result {
        Ok(true) => Ok(()),
        Ok(false) => {
            if let Err(error) = direct_result {
                bail!(
                    "taskkill /T for child {pid} returned non-zero; direct-child fallback also failed: {error}"
                );
            }
            bail!(
                "taskkill /T for child {pid} returned non-zero; descendant cleanup is unverified"
            );
        }
        Err(error) => {
            if let Err(direct_error) = direct_result {
                bail!(
                    "failed to start taskkill /T for child {pid}: {error}; direct-child fallback also failed: {direct_error}"
                );
            }
            bail!(
                "failed to start taskkill /T for child {pid}: {error}; descendant cleanup is unverified"
            );
        }
    }
}

#[cfg(unix)]
fn terminate_owned_processes(child: &mut Child) -> Result<()> {
    let group = match UnixProcessGroup::for_child(child) {
        Ok(group) => group,
        Err(error) => {
            let _ = child.kill();
            return Err(error.into());
        }
    };
    let term_error = group.signal(libc::SIGTERM).err();
    thread::sleep(Duration::from_millis(500));
    let kill_error = group.signal(libc::SIGKILL).err();
    let _ = child.kill();
    match term_error.or(kill_error) {
        Some(error) => Err(error.into()),
        None => Ok(()),
    }
}

#[cfg(not(any(unix, windows)))]
fn terminate_owned_processes(child: &mut Child) -> Result<()> {
    child
        .kill()
        .context("failed to terminate timed-out direct child")
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy)]
struct UnixProcessGroup {
    id: libc::pid_t,
}

#[cfg(unix)]
impl UnixProcessGroup {
    fn for_child(child: &Child) -> io::Result<Self> {
        let child_pid = libc::pid_t::try_from(child.id()).map_err(pid_conversion_error)?;
        // SAFETY: getpgid only reads kernel process metadata for the live child PID.
        let child_group = unsafe { libc::getpgid(child_pid) };
        if child_group == -1 {
            return Err(io::Error::last_os_error());
        }
        let parent_group = current_process_group()?;
        if child_group <= 1 || child_group != child_pid || child_group == parent_group {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "child process group {child_group} is not an isolated child group for pid {child_pid} (runner group {parent_group})"
                ),
            ));
        }
        Ok(Self { id: child_group })
    }

    #[cfg(test)]
    fn id(self) -> libc::pid_t {
        self.id
    }

    fn signal(self, signal: libc::c_int) -> io::Result<()> {
        // SAFETY: the negative, previously validated PGID targets only the isolated child group.
        if unsafe { libc::kill(-self.id, signal) } == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            Ok(())
        } else {
            Err(error)
        }
    }
}

#[cfg(unix)]
fn current_process_group() -> io::Result<libc::pid_t> {
    // SAFETY: getpgrp has no arguments and only reads the caller's process group.
    let group = unsafe { libc::getpgrp() };
    if group <= 1 {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("runner process group {group} is not signal-safe"),
        ))
    } else {
        Ok(group)
    }
}

#[cfg(unix)]
fn pid_conversion_error(error: TryFromIntError) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("child PID is outside pid_t range: {error}"),
    )
}

#[cfg(all(test, unix))]
#[path = "process/tests.rs"]
mod tests;

#[cfg(all(test, windows))]
#[path = "process/windows_tests.rs"]
mod windows_tests;
