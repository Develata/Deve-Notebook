//! plan_ref:
//!   - 11_ui_design/index#native-post-gate-common-contract
//!   - 11_ui_design/02_desktop#desktop-native-shell-modes
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!
//! Native local backend assembly shared by Desktop and Mobile shells.

use crate::server::{NativeLoopbackAuthMaterial, ServerLaunchOptions, start_server_with_options};
use anyhow::Context;
use deve_core::config::{AppProfile, GitBridgeMode, P2pConfig, SyncMode};
use deve_core::ledger::RepoManager;
use deve_core::ledger::init::RepoInitOptions;
use deve_core::plugin::loader::PluginLoader;
use deve_core::plugin::runtime::PluginRuntime;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const NATIVE_DEFAULT_REPO_NAME: &str = "default";
const DEVE_PLUGIN_DIR_ENV: &str = "DEVE_PLUGIN_DIR";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLocalBackendLayout {
    pub app_data_dir: PathBuf,
    pub ledger_dir: PathBuf,
    pub projection_base: PathBuf,
    pub workspace_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLocalBackendOptions {
    pub app_data_dir: PathBuf,
    pub port: u16,
    pub snapshot_depth: usize,
    pub profile: AppProfile,
    pub sync_mode: SyncMode,
    pub git_bridge: GitBridgeMode,
    pub p2p: P2pConfig,
    pub session_bound: bool,
    pub auth_material: Option<NativeLoopbackAuthMaterial>,
}

impl NativeLocalBackendOptions {
    pub fn new(app_data_dir: impl Into<PathBuf>, port: u16) -> Self {
        Self {
            app_data_dir: app_data_dir.into(),
            port,
            snapshot_depth: 100,
            profile: AppProfile::Standard,
            sync_mode: SyncMode::Auto,
            git_bridge: GitBridgeMode::Mirror,
            p2p: P2pConfig::default(),
            session_bound: false,
            auth_material: None,
        }
    }

    pub fn with_auth_material(mut self, auth_material: NativeLoopbackAuthMaterial) -> Self {
        self.auth_material = Some(auth_material);
        self
    }
}

pub fn init_default_native_backend(
    app_data_dir: &Path,
    snapshot_depth: usize,
) -> anyhow::Result<(Arc<RepoManager>, NativeLocalBackendLayout)> {
    std::fs::create_dir_all(app_data_dir)
        .with_context(|| format!("Failed to create native app data dir {app_data_dir:?}"))?;
    let app_data_dir = std::fs::canonicalize(app_data_dir)
        .with_context(|| format!("Failed to canonicalize native app data dir {app_data_dir:?}"))?;
    let ledger_dir = app_data_dir.join("ledger");
    let projection_base = app_data_dir.join("workspace");
    std::fs::create_dir_all(&projection_base)
        .with_context(|| format!("Failed to create native projection base {projection_base:?}"))?;

    let repo = RepoManager::init_with_options(
        &ledger_dir,
        snapshot_depth,
        Some(NATIVE_DEFAULT_REPO_NAME),
        RepoInitOptions {
            repo_id: None,
            repo_url: None,
        },
    )?;
    repo.set_projection_base_for_local_repo(NATIVE_DEFAULT_REPO_NAME, &projection_base)?;
    let workspace_root = repo.local_repo_workspace_root(NATIVE_DEFAULT_REPO_NAME)?;
    std::fs::create_dir_all(&workspace_root)
        .with_context(|| format!("Failed to create native workspace root {workspace_root:?}"))?;
    let workspace_root = repo.ensure_local_repo_workspace_identity(NATIVE_DEFAULT_REPO_NAME)?;
    deve_core::utils::notegit::ensure_gitignore_ignores_notegit(&workspace_root)?;
    std::fs::create_dir_all(deve_core::utils::notegit::host_keys_dir(&ledger_dir))
        .with_context(|| format!("Failed to create native host key dir under {ledger_dir:?}"))?;
    repo.validate_projection_locator_map()?;

    Ok((
        Arc::new(repo),
        NativeLocalBackendLayout {
            app_data_dir,
            ledger_dir,
            projection_base,
            workspace_root,
        },
    ))
}

pub async fn start_native_loopback_backend(
    options: NativeLocalBackendOptions,
) -> anyhow::Result<()> {
    let (repo, _) = init_default_native_backend(&options.app_data_dir, options.snapshot_depth)?;
    let plugins = load_native_plugins()?;
    let launch = match options.auth_material {
        Some(auth_material) => ServerLaunchOptions::native_loopback_with_auth_material(
            options.port,
            options.session_bound,
            auth_material,
        ),
        None => ServerLaunchOptions::native_loopback(options.port, options.session_bound),
    };
    start_server_with_options(
        repo,
        launch,
        plugins,
        options.profile,
        options.sync_mode,
        options.git_bridge,
        options.p2p,
    )
    .await
}

pub fn load_native_plugins() -> anyhow::Result<Vec<Box<dyn PluginRuntime>>> {
    let plugin_dir = resolve_native_plugin_dir()?;
    let loader = PluginLoader::new(plugin_dir.clone());
    let plugins = loader.load_all_strict()?;
    tracing::info!(?plugin_dir, "Loaded {} native plugins.", plugins.len());
    Ok(plugins)
}

fn resolve_native_plugin_dir() -> anyhow::Result<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(value) = std::env::var_os(DEVE_PLUGIN_DIR_ENV).filter(|value| !value.is_empty()) {
        candidates.push(PathBuf::from(value));
    }
    candidates.extend(default_native_plugin_dir_candidates());

    for candidate in candidates {
        if candidate.try_exists().map_err(|source| {
            anyhow::anyhow!(
                "Failed to stat native plugin directory {:?}: {source}",
                candidate
            )
        })? {
            return Ok(candidate);
        }
    }

    Ok(PathBuf::from("plugins"))
}

fn default_native_plugin_dir_candidates() -> Vec<PathBuf> {
    let mut candidates = vec![PathBuf::from("plugins")];
    if let Some(exe_parent) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
    {
        candidates.push(exe_parent.join("plugins"));
        candidates.push(exe_parent.join("..").join("..").join("plugins"));
    }
    candidates.push(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("plugins"),
    );
    candidates
}

#[cfg(test)]
mod tests {
    use super::{NATIVE_DEFAULT_REPO_NAME, NativeLocalBackendOptions, init_default_native_backend};

    #[test]
    fn native_default_backend_initializes_repo_projection_and_notegit() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (repo, layout) =
            init_default_native_backend(dir.path(), 8).expect("init native backend");

        assert_eq!(
            layout.app_data_dir,
            std::fs::canonicalize(dir.path()).expect("canonical")
        );
        assert!(layout.ledger_dir.join("local").is_dir());
        assert!(layout.projection_base.is_dir());
        assert!(layout.workspace_root.is_dir());
        assert!(
            layout
                .workspace_root
                .join(".notegit/identity.toml")
                .is_file()
        );

        let gitignore =
            std::fs::read_to_string(layout.workspace_root.join(".gitignore")).expect("gitignore");
        assert!(gitignore.lines().any(|line| line.trim() == ".notegit/"));
        assert!(
            repo.get_repo_info_for(None, Some(NATIVE_DEFAULT_REPO_NAME))
                .expect("repo lookup")
                .is_some()
        );
    }

    #[test]
    fn native_local_backend_options_default_to_local_runtime_contract() {
        let options = NativeLocalBackendOptions::new("native-data", 39111);

        assert_eq!(options.port, 39111);
        assert_eq!(options.snapshot_depth, 100);
        assert!(!options.session_bound);
        assert!(options.auth_material.is_none());
        assert!(!options.p2p.enabled);
    }
}
