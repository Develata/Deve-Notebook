//! Bounded producer process and process-tree lifecycle.
//! plan_ref: 18_release#first-tag-acceptance-matrix

use super::model::CommandStep;
use anyhow::{Context, Result, bail};
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
            terminate_process_tree(&mut child);
            if !wait_for_exit(&mut child, TERMINATION_WAIT)? {
                bail!(
                    "command step {} exceeded {} seconds and its process tree did not terminate within {} seconds",
                    step.program,
                    timeout.as_secs(),
                    TERMINATION_WAIT.as_secs()
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
fn terminate_process_tree(child: &mut Child) {
    let _ = Command::new("taskkill")
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .status();
    let _ = child.kill();
}

#[cfg(unix)]
fn terminate_process_tree(child: &mut Child) {
    let process_group = format!("-{}", child.id());
    let _ = Command::new("kill")
        .args(["-TERM", &process_group])
        .status();
    thread::sleep(Duration::from_millis(500));
    let _ = Command::new("kill")
        .args(["-KILL", &process_group])
        .status();
    let _ = child.kill();
}

#[cfg(not(any(unix, windows)))]
fn terminate_process_tree(child: &mut Child) {
    let _ = child.kill();
}
