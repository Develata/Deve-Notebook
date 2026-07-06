mod env_policy;
mod policy;
mod session;
mod spawn_spec;

use super::{
    NativeEndpointReady, NativeProcessBindHints, NativeProcessEnvBinding,
    NativeProcessPathResolution, NativeProcessSpawnSpec,
};

fn valid_spawn_spec() -> NativeProcessSpawnSpec {
    let root = std::env::current_dir().expect("current dir");
    NativeProcessSpawnSpec {
        executable: root.join("target/native/deve_cli"),
        argv: vec!["serve".to_string(), "--dev".to_string()],
        cwd: root.clone(),
        env_allowlist: vec!["DEVE_PROFILE".to_string(), "MEM_CACHE_MB".to_string()],
        env: vec![NativeProcessEnvBinding {
            key: "DEVE_PROFILE".to_string(),
            value: "standard".to_string(),
        }],
        profile: "standard".to_string(),
        config_path: root.join("config.toml"),
        ledger_path: root.join("ledger"),
        bind_hints: NativeProcessBindHints {
            http_host: "127.0.0.1".to_string(),
            http_port: Some(3001),
            ws_host: "localhost".to_string(),
            ws_port: Some(3001),
        },
        path_resolution: NativeProcessPathResolution::AbsoluteOnly,
    }
}

fn endpoint(http_base: &str, ws_base: &str) -> NativeEndpointReady {
    NativeEndpointReady {
        http_base: http_base.to_string(),
        ws_base: ws_base.to_string(),
        node_role: "native-main".to_string(),
        session_bound: true,
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
