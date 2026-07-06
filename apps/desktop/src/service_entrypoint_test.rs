use std::path::PathBuf;

use deve_core::config::AppProfile;

use crate::DesktopLocalServiceEntrypointInput;

mod data_root;
mod env_policy;
mod spawn_spec;

fn abs(path: &str) -> PathBuf {
    let root = std::env::current_dir().expect("current dir");
    root.join(path)
}

fn input() -> DesktopLocalServiceEntrypointInput {
    DesktopLocalServiceEntrypointInput {
        current_exe: abs("target/debug/deve_desktop"),
        data_root: abs("desktop-data"),
        port: 39101,
        profile: AppProfile::LowSpec,
    }
}

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct EnvGuard {
    old: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn set(vars: &[(&'static str, Option<&str>)]) -> Self {
        let old = vars
            .iter()
            .map(|(key, _)| (*key, std::env::var(key).ok()))
            .collect::<Vec<_>>();
        for (key, value) in vars {
            // SAFETY: tests serialize env mutation through ENV_LOCK and restore every key.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
        Self { old }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.old.drain(..) {
            // SAFETY: EnvGuard owns restoration for keys it changed while ENV_LOCK is held.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}
