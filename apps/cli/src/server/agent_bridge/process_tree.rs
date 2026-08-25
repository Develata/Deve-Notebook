//! plan_ref:
//!   - 16_ai_agent#trusted-agent-bridge
//!
//! Host-owned process-tree containment for the optional Trusted CLI bridge.

use std::io;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::{Child, ChildStdout, Command};

const CHILD_CLEANUP_TIMEOUT: Duration = Duration::from_secs(1);

pub(super) struct ContainedChild {
    child: Option<Child>,
    tree: Option<ProcessTree>,
    retired: bool,
}

impl ContainedChild {
    pub(super) fn spawn(command: &mut Command) -> io::Result<Self> {
        configure_before_spawn(command)?;
        let mut child = command.spawn()?;
        let tree = match ProcessTree::attach(&child) {
            Ok(tree) => tree,
            Err(error) => {
                let _ = child.start_kill();
                spawn_bounded_reaper(child, None);
                return Err(io::Error::new(
                    error.kind(),
                    format!("Agent CLI process-tree containment failed: {error}"),
                ));
            }
        };
        Ok(Self {
            child: Some(child),
            tree: Some(tree),
            retired: false,
        })
    }

    pub(super) fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.child.as_mut()?.stdout.take()
    }

    pub(super) async fn wait(&mut self) -> io::Result<std::process::ExitStatus> {
        self.child
            .as_mut()
            .ok_or_else(|| io::Error::other("Agent CLI child is already retired"))?
            .wait()
            .await
    }

    pub(super) async fn retire_tree(&mut self) -> io::Result<()> {
        if self.retired {
            return Ok(());
        }
        let mut cleanup_error = None;
        if let Some(tree) = self.tree.as_ref()
            && let Err(error) = tree.terminate()
        {
            cleanup_error = Some(io::Error::new(
                error.kind(),
                format!("failed to terminate Agent CLI process tree: {error}"),
            ));
        }
        let Some(child) = self.child.as_mut() else {
            self.retired = true;
            return cleanup_error.map_or(Ok(()), Err);
        };
        let _ = child.start_kill();
        match tokio::time::timeout(CHILD_CLEANUP_TIMEOUT, child.wait()).await {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                cleanup_error.get_or_insert_with(|| {
                    io::Error::new(
                        error.kind(),
                        format!("failed to reap Agent CLI process tree: {error}"),
                    )
                });
            }
            Err(_) => {
                cleanup_error.get_or_insert_with(|| {
                    io::Error::new(
                        io::ErrorKind::TimedOut,
                        "timed out reaping Agent CLI process tree",
                    )
                });
            }
        }
        self.retired = true;
        cleanup_error.map_or(Ok(()), Err)
    }

    pub(super) fn retire_tree_after_wait(&mut self) -> io::Result<()> {
        if self.retired {
            return Ok(());
        }
        let result = self.tree.as_ref().map_or(Ok(()), ProcessTree::terminate);
        self.retired = true;
        result.map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("failed to retire Agent CLI descendants: {error}"),
            )
        })
    }
}

impl Drop for ContainedChild {
    fn drop(&mut self) {
        if self.retired {
            return;
        }
        let tree = self.tree.take();
        if let Some(tree) = tree.as_ref() {
            let _ = tree.terminate();
        }
        let Some(mut child) = self.child.take() else {
            return;
        };
        let _ = child.start_kill();
        spawn_bounded_reaper(child, tree);
    }
}

fn spawn_bounded_reaper(mut child: Child, tree: Option<ProcessTree>) {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        std::mem::drop(handle.spawn(async move {
            let _tree = tree;
            match tokio::time::timeout(CHILD_CLEANUP_TIMEOUT, child.wait()).await {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => {
                    tracing::warn!(category = "agent_process_tree_drop_wait_failed", %error)
                }
                Err(_) => tracing::warn!(category = "agent_process_tree_drop_wait_timeout"),
            }
        }));
        return;
    }

    std::mem::drop(std::thread::spawn(move || {
        let _tree = tree;
        let deadline = std::time::Instant::now() + CHILD_CLEANUP_TIMEOUT;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Ok(None) => {
                    tracing::warn!(category = "agent_process_tree_drop_wait_timeout");
                    return;
                }
                Err(error) => {
                    tracing::warn!(category = "agent_process_tree_drop_wait_failed", %error);
                    return;
                }
            }
        }
    }));
}

#[cfg(unix)]
fn configure_before_spawn(command: &mut Command) -> io::Result<()> {
    use std::os::unix::process::CommandExt;
    command.as_std_mut().process_group(0);
    Ok(())
}

#[cfg(windows)]
fn configure_before_spawn(_command: &mut Command) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Trusted CLI is unavailable on Windows until creation-time Job Object containment is implemented",
    ))
}

#[cfg(not(any(unix, windows)))]
fn configure_before_spawn(_command: &mut Command) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Agent CLI process-tree containment is unavailable on this platform",
    ))
}

#[cfg(unix)]
struct ProcessTree {
    process_group: libc::pid_t,
}

#[cfg(unix)]
impl ProcessTree {
    fn attach(child: &Child) -> io::Result<Self> {
        let pid = child
            .id()
            .ok_or_else(|| io::Error::other("Agent CLI child has no process id"))?;
        let process_group = libc::pid_t::try_from(pid)
            .map_err(|_| io::Error::other("Agent CLI process id exceeds platform range"))?;
        Ok(Self { process_group })
    }

    fn terminate(&self) -> io::Result<()> {
        let result = unsafe { libc::kill(-self.process_group, libc::SIGKILL) };
        if result == 0 {
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

#[cfg(not(unix))]
struct ProcessTree;

#[cfg(not(unix))]
impl ProcessTree {
    fn attach(_child: &Child) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Agent CLI process-tree containment is unavailable on this platform",
        ))
    }

    fn terminate(&self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn trusted_cli_command(cli_path: &str, query: &str) -> Command {
    let mut command = Command::new(cli_path);
    command
        .env_clear()
        .args(["run", query])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command
}
