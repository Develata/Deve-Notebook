//! plan_ref:
//!   - 17_plugins#plugin-runtime-boundary
//!
use deve_core::ledger::RepoManager;
use deve_core::plugin::loader::PluginLoader;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const DEVE_PLUGIN_DIR_ENV: &str = "DEVE_PLUGIN_DIR";

pub(super) fn init_runtime(
    ledger_dir: &PathBuf,
    vault_path: &PathBuf,
    snapshot_depth: usize,
) -> anyhow::Result<Arc<RepoManager>> {
    let mut repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    repo.set_vault_root_checked(vault_path)?;
    Ok(Arc::new(repo))
}

pub(super) fn load_plugins()
-> anyhow::Result<Vec<Box<dyn deve_core::plugin::runtime::PluginRuntime>>> {
    let plugin_dir = resolve_plugin_dir()?;
    let loader = PluginLoader::new(plugin_dir.clone());
    let plugins = loader.load_all_strict()?;
    tracing::info!(?plugin_dir, "Loaded {} plugins.", plugins.len());
    Ok(plugins)
}

fn resolve_plugin_dir() -> anyhow::Result<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(value) = std::env::var_os(DEVE_PLUGIN_DIR_ENV).filter(|value| !value.is_empty()) {
        let path = PathBuf::from(value);
        candidates.push(path);
    }
    candidates.extend(default_plugin_dir_candidates());

    for candidate in candidates {
        if candidate.try_exists().map_err(|source| {
            anyhow::anyhow!("Failed to stat plugin directory {:?}: {source}", candidate)
        })? {
            return Ok(candidate);
        }
    }

    Ok(PathBuf::from("plugins"))
}

fn default_plugin_dir_candidates() -> Vec<PathBuf> {
    default_plugin_dir_candidates_for(
        std::env::current_exe().ok().as_deref(),
        Path::new(env!("CARGO_MANIFEST_DIR")),
    )
}

fn default_plugin_dir_candidates_for(
    current_exe: Option<&Path>,
    manifest_dir: &Path,
) -> Vec<PathBuf> {
    let mut candidates = vec![PathBuf::from("plugins")];

    if let Some(exe_parent) = current_exe.and_then(Path::parent) {
        candidates.push(exe_parent.join("plugins"));
        candidates.push(exe_parent.join("..").join("..").join("plugins"));
    }

    candidates.push(manifest_dir.join("..").join("..").join("plugins"));
    candidates
}

pub(super) fn find_free_port(start: u16, span: u16) -> Option<u16> {
    for p in start..=start.saturating_add(span) {
        let addr = format!("127.0.0.1:{p}");
        if TcpListener::bind(&addr).is_ok() {
            return Some(p);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{DEVE_PLUGIN_DIR_ENV, default_plugin_dir_candidates_for, resolve_plugin_dir};
    use std::ffi::{OsStr, OsString};
    use std::path::Path;
    use std::sync::{LazyLock, Mutex};

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: &OsStr) -> Self {
            let previous = std::env::var_os(key);
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            unsafe {
                if let Some(previous) = self.previous.as_ref() {
                    std::env::set_var(self.key, previous);
                } else {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    #[test]
    fn default_plugin_dir_candidates_include_cwd_sibling_and_workspace_fallback() {
        let candidates = default_plugin_dir_candidates_for(
            Some(Path::new("C:/apps/deve/deve_cli.exe")),
            Path::new("E:/repo/apps/cli"),
        );

        assert_eq!(candidates[0], Path::new("plugins"));
        assert!(
            candidates
                .iter()
                .any(|path| path == Path::new("C:/apps/deve/plugins"))
        );
        assert!(
            candidates
                .iter()
                .any(|path| path == Path::new("C:/apps/deve/../../plugins"))
        );
        assert!(
            candidates
                .iter()
                .any(|path| path == Path::new("E:/repo/apps/cli/../../plugins"))
        );
    }

    #[test]
    fn resolve_plugin_dir_finds_bundled_ai_chat_in_development_workspace() {
        let _lock = ENV_LOCK.lock().expect("plugin env lock");
        let plugin_dir = resolve_plugin_dir().expect("plugin dir");

        assert!(
            plugin_dir.join("ai-chat").join("manifest.json").exists(),
            "resolved plugin dir should contain bundled ai-chat: {:?}",
            plugin_dir
        );
    }

    #[test]
    fn resolve_plugin_dir_skips_missing_env_candidate() {
        let _lock = ENV_LOCK.lock().expect("plugin env lock");
        let missing = std::env::temp_dir().join("deve-missing-plugin-dir-for-test");
        let _env = EnvVarGuard::set(DEVE_PLUGIN_DIR_ENV, missing.as_os_str());
        let plugin_dir = resolve_plugin_dir().expect("plugin dir");

        assert!(
            plugin_dir.join("ai-chat").join("manifest.json").exists(),
            "missing DEVE_PLUGIN_DIR should fall back to bundled ai-chat: {:?}",
            plugin_dir
        );
    }
}
