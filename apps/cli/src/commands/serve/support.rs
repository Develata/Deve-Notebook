//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout
//!   - 03_storage/projection#projection-locator-contract
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-scope-runtime
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 16_ai_agent#native-ai-chat-runtime
//!   - 19_plugins#plugin-runtime-boundary
//!
use deve_core::ledger::RepoManager;
use deve_core::plugin::loader::PluginLoader;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const DEVE_PLUGIN_DIR_ENV: &str = "DEVE_PLUGIN_DIR";
pub(super) fn init_runtime(
    ledger_dir: &Path,
    snapshot_depth: usize,
) -> anyhow::Result<Arc<RepoManager>> {
    let repo_ids = if ledger_dir.exists() {
        deve_core::ledger::normal_catalog_ids_for_ledger(ledger_dir)?
    } else {
        Vec::new()
    };
    let repo = match repo_ids.first().copied() {
        Some(repo_id) => {
            RepoManager::init_existing_for_repo_id(ledger_dir, snapshot_depth, repo_id)?
        }
        None => RepoManager::init_empty_host(ledger_dir, snapshot_depth)?,
    };
    repo.validate_projection_locator_map()?;
    Ok(Arc::new(repo))
}

pub(super) fn load_plugins()
-> anyhow::Result<Vec<Box<dyn deve_core::plugin::runtime::PluginRuntime>>> {
    let plugin_dir = resolve_plugin_dir()?;
    let loader = PluginLoader::new(plugin_dir.clone());
    let mut external_plugins = loader.load_all_strict()?;
    if is_builtin_source_plugin_dir(&plugin_dir) && !explicit_plugin_dir_selected(&plugin_dir) {
        external_plugins.retain(|plugin| plugin.manifest().id != "ai-chat");
    }
    let plugins = crate::server::ai_chat::assemble_runtime_plugins(external_plugins)?;
    tracing::info!(?plugin_dir, "Loaded {} plugins.", plugins.len());
    Ok(plugins)
}

fn is_builtin_source_plugin_dir(plugin_dir: &Path) -> bool {
    let builtin_source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("plugins");
    let Ok(plugin_dir) = plugin_dir.canonicalize() else {
        return false;
    };
    builtin_source
        .canonicalize()
        .is_ok_and(|builtin_source| plugin_dir == builtin_source)
}

fn explicit_plugin_dir_selected(plugin_dir: &Path) -> bool {
    let Some(configured) = std::env::var_os(DEVE_PLUGIN_DIR_ENV).filter(|value| !value.is_empty())
    else {
        return false;
    };
    let Ok(configured) = PathBuf::from(configured).canonicalize() else {
        return false;
    };
    plugin_dir
        .canonicalize()
        .is_ok_and(|plugin_dir| plugin_dir == configured)
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
    use super::{
        DEVE_PLUGIN_DIR_ENV, default_plugin_dir_candidates_for, load_plugins, resolve_plugin_dir,
    };
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

    #[test]
    fn serve_loader_registers_builtin_ai_without_external_plugins() {
        let _lock = ENV_LOCK.lock().expect("plugin env lock");
        let empty_plugins = tempfile::tempdir().expect("empty plugin dir");
        let _env = EnvVarGuard::set(DEVE_PLUGIN_DIR_ENV, empty_plugins.path().as_os_str());

        let plugins = load_plugins().expect("serve plugins");

        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].manifest().id, "ai-chat");
    }

    #[test]
    fn serve_loader_rejects_explicit_external_ai_chat_duplicate() {
        let _lock = ENV_LOCK.lock().expect("plugin env lock");
        let source_plugins = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("plugins");
        let _env = EnvVarGuard::set(DEVE_PLUGIN_DIR_ENV, source_plugins.as_os_str());

        let error = match load_plugins() {
            Ok(_) => panic!("explicit duplicate ai-chat must fail closed"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("Duplicate plugin id 'ai-chat'"));
    }
}
