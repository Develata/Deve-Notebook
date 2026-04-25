//! plan_ref:
//!   - 17_plugins#plugin-runtime-boundary
//!
use deve_core::ledger::RepoManager;
use deve_core::plugin::loader::PluginLoader;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;

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
    let loader = PluginLoader::new(PathBuf::from("plugins"));
    let plugins = loader.load_all_strict()?;
    tracing::info!("Loaded {} plugins.", plugins.len());
    Ok(plugins)
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
