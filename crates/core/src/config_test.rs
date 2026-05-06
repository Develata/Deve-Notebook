use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[path = "config_agent_bridge_test.rs"]
mod agent_bridge_tests;
#[path = "config_load_test.rs"]
mod load_tests;

static CWD_LOCK: Mutex<()> = Mutex::new(());

struct CwdGuard {
    old_cwd: PathBuf,
}

impl CwdGuard {
    fn enter(path: impl AsRef<Path>) -> Self {
        let old_cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(path).expect("set cwd");
        Self { old_cwd }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.old_cwd).expect("restore cwd");
    }
}

struct EnvGuard {
    previous: Vec<(&'static str, Option<OsString>)>,
}

impl EnvGuard {
    fn set(entries: &[(&'static str, &'static str)]) -> Self {
        let previous = entries
            .iter()
            .map(|(key, value)| {
                let old = std::env::var_os(key);
                unsafe {
                    std::env::set_var(key, value);
                }
                (*key, old)
            })
            .collect();
        Self { previous }
    }

    fn set_optional(entries: &[(&'static str, Option<&str>)]) -> Self {
        let previous = entries
            .iter()
            .map(|(key, value)| {
                let old = std::env::var_os(key);
                unsafe {
                    match value {
                        Some(value) => std::env::set_var(key, value),
                        None => std::env::remove_var(key),
                    }
                }
                (*key, old)
            })
            .collect();
        Self { previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.previous.drain(..) {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}
