use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

mod agent_bridge_tests;
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
    previous: Vec<(OsString, Option<OsString>)>,
}

impl EnvGuard {
    fn set_optional(entries: &[(&'static str, Option<&str>)]) -> Self {
        let mut keys = std::env::vars_os()
            .filter_map(|(key, _)| {
                let is_deve = key.to_str().is_some_and(|key| key.starts_with("DEVE_"));
                is_deve.then_some(key)
            })
            .collect::<Vec<_>>();
        keys.extend([
            OsString::from("MEM_CACHE_MB"),
            OsString::from("AGENT_CLI_PATH"),
        ]);
        keys.extend(entries.iter().map(|(key, _)| OsString::from(key)));

        let mut previous = Vec::new();
        for key in keys {
            if previous.iter().any(|(seen, _)| seen == &key) {
                continue;
            }
            let key_str = key.to_string_lossy();
            let value = entries
                .iter()
                .find_map(|(entry_key, value)| (key_str == *entry_key).then_some(*value))
                .flatten();
            let old = std::env::var_os(&key);
            // SAFETY: config tests serialize env mutation through CWD_LOCK and restore every key.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(&key, value),
                    None => std::env::remove_var(&key),
                }
            }
            previous.push((key, old));
        }
        Self { previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.previous.drain(..) {
            // SAFETY: EnvGuard owns restoration for keys it changed while tests hold CWD_LOCK.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}
