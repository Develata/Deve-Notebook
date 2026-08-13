//! plan_ref:
//!   - 11_ui_design/index#native-post-gate-common-contract
//!   - 11_ui_design/02_desktop#desktop-native-shell-modes
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!
//! Native local backend assembly shared by Desktop and Mobile shells.

use crate::server::{
    EmbeddedServerRuntime, NativeLoopbackAuthMaterial, ServerLaunchOptions, ServerTransportRuntime,
    ServerTransportServeError,
};
use anyhow::Context;
use deve_core::config::{AppProfile, P2pConfig, SyncMode};
use deve_core::ledger::RepoManager;
use deve_core::plugin::loader::PluginLoader;
use deve_core::plugin::runtime::PluginRuntime;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

const DEVE_PLUGIN_DIR_ENV: &str = "DEVE_PLUGIN_DIR";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLocalBackendLayout {
    pub app_data_dir: PathBuf,
    pub ledger_dir: PathBuf,
    pub projection_base: PathBuf,
    pub workspace_root: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLocalBackendOptions {
    pub app_data_dir: PathBuf,
    pub port: u16,
    pub snapshot_depth: usize,
    pub profile: AppProfile,
    pub sync_mode: SyncMode,
    pub p2p: P2pConfig,
    pub session_bound: bool,
    pub auth_material: Option<NativeLoopbackAuthMaterial>,
    pub prewarm_enabled: bool,
    pub native_ai_enabled: bool,
}

#[derive(Debug)]
pub struct NativeLoopbackListener {
    listener: TcpListener,
    port: u16,
}

pub struct NativeEmbeddedServerRuntime {
    runtime: Option<EmbeddedServerRuntime>,
}

#[derive(Debug)]
pub struct NativeEmbeddedTransportError {
    message: String,
    sessions_retired: bool,
}

impl NativeEmbeddedTransportError {
    pub fn sessions_retired(&self) -> bool {
        self.sessions_retired
    }

    fn before_serve(error: anyhow::Error) -> Self {
        Self {
            message: error.to_string(),
            sessions_retired: true,
        }
    }
}

impl std::fmt::Display for NativeEmbeddedTransportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for NativeEmbeddedTransportError {}

impl From<ServerTransportServeError> for NativeEmbeddedTransportError {
    fn from(error: ServerTransportServeError) -> Self {
        Self {
            sessions_retired: error.sessions_retired(),
            message: error.to_string(),
        }
    }
}

#[derive(Clone)]
pub struct NativeEmbeddedTransportRuntime {
    transport: ServerTransportRuntime,
}

impl NativeLoopbackListener {
    pub fn port(&self) -> u16 {
        self.port
    }

    fn into_tokio_listener(self) -> std::io::Result<tokio::net::TcpListener> {
        self.listener.set_nonblocking(true)?;
        tokio::net::TcpListener::from_std(self.listener)
    }
}

impl NativeLocalBackendOptions {
    pub fn new(app_data_dir: impl Into<PathBuf>, port: u16) -> Self {
        Self {
            app_data_dir: app_data_dir.into(),
            port,
            snapshot_depth: 100,
            profile: AppProfile::Standard,
            sync_mode: SyncMode::Auto,
            p2p: P2pConfig::default(),
            session_bound: false,
            auth_material: None,
            prewarm_enabled: true,
            native_ai_enabled: deve_core::config::Config::load().ai.native_enabled,
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

    // Bootstrap decision comes from durable catalog membership, probed without
    // creating any repo database. A bare `RepoManager::init` repo would stay
    // uncataloged and therefore invisible to catalog-backed resolution. A
    // missing ledger root is a first boot: zero cataloged repos.
    let existing = if ledger_dir.exists() {
        deve_core::ledger::normal_catalog_ids_for_ledger(&ledger_dir)?
    } else {
        Vec::new()
    };
    let (repo, workspace_root) = match existing.first().copied() {
        None => (
            RepoManager::init_empty_host(&ledger_dir, snapshot_depth)?,
            None,
        ),
        Some(repo_id) => {
            let repo =
                RepoManager::init_existing_for_repo_id(&ledger_dir, snapshot_depth, repo_id)?;
            let workspace_root =
                repo.check_projection_locator_for_local_repo(&repo_id.to_string())?;
            (repo, Some(workspace_root))
        }
    };
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
    let listener = bind_native_loopback_listener_exact(options.port)
        .with_context(|| format!("Failed to bind native loopback port {}", options.port))?;
    start_native_loopback_backend_with_listener(options, listener).await
}

pub async fn start_native_loopback_backend_with_listener(
    options: NativeLocalBackendOptions,
    listener: NativeLoopbackListener,
) -> anyhow::Result<()> {
    start_native_loopback_backend_with_listener_until_shutdown(
        options,
        listener,
        std::future::pending(),
    )
    .await
}

pub async fn start_native_loopback_backend_with_listener_until_shutdown<F>(
    mut options: NativeLocalBackendOptions,
    listener: NativeLoopbackListener,
    shutdown: F,
) -> anyhow::Result<()>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    options.port = listener.port();
    let runtime = NativeEmbeddedServerRuntime::initialize(&options).await?;
    let transport = runtime.transport();
    let serve_result = transport
        .serve_with_listener_until_shutdown(options, listener, shutdown)
        .await;
    let shutdown_result = runtime.shutdown(Duration::from_secs(5)).await;
    match (serve_result, shutdown_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error.into()),
        (Ok(()), Err(error)) => Err(error),
        (Err(serve_error), Err(shutdown_error)) => Err(anyhow::Error::new(serve_error).context(
            format!("native embedded runtime shutdown also failed: {shutdown_error}"),
        )),
    }
}

impl NativeEmbeddedServerRuntime {
    pub async fn initialize(options: &NativeLocalBackendOptions) -> anyhow::Result<Self> {
        let mut config = deve_core::config::Config::load();
        config.ai.native_enabled = options.native_ai_enabled;
        crate::server::ai_chat::init_from_config(&config);
        crate::server::agent_bridge::init_from_config(&config);
        let (repo, _) = init_default_native_backend(&options.app_data_dir, options.snapshot_depth)?;
        let ai_settings_root =
            native_ai_provider_settings_root(&options.app_data_dir, cfg!(target_os = "android"));
        std::fs::create_dir_all(&ai_settings_root).with_context(|| {
            format!("Failed to create native AI settings root {ai_settings_root:?}")
        })?;
        let plugins = load_native_plugins(&options.app_data_dir, options.native_ai_enabled)?;
        let launch = native_server_launch_options(options)
            .with_ai_provider_settings_data_root(Some(ai_settings_root));
        let runtime = EmbeddedServerRuntime::initialize(
            repo,
            &launch,
            plugins,
            options.profile,
            options.sync_mode,
            options.p2p.clone(),
            options.prewarm_enabled,
        )?;
        Ok(Self {
            runtime: Some(runtime),
        })
    }

    pub fn transport(&self) -> NativeEmbeddedTransportRuntime {
        NativeEmbeddedTransportRuntime {
            transport: self
                .runtime
                .as_ref()
                .expect("native embedded runtime is present before shutdown")
                .transport(),
        }
    }

    pub async fn shutdown(mut self, timeout: Duration) -> anyhow::Result<()> {
        let Some(runtime) = self.runtime.take() else {
            return Ok(());
        };
        runtime.shutdown(timeout).await
    }
}

impl NativeEmbeddedTransportRuntime {
    pub async fn serve_with_listener_until_shutdown<F>(
        &self,
        mut options: NativeLocalBackendOptions,
        listener: NativeLoopbackListener,
        shutdown: F,
    ) -> Result<(), NativeEmbeddedTransportError>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        options.port = listener.port();
        let listener = listener
            .into_tokio_listener()
            .context("Failed to prepare native loopback listener")
            .map_err(NativeEmbeddedTransportError::before_serve)?;
        self.transport
            .serve(listener, native_server_launch_options(&options), shutdown)
            .await
            .map_err(NativeEmbeddedTransportError::from)
    }
}

fn native_server_launch_options(options: &NativeLocalBackendOptions) -> ServerLaunchOptions {
    let launch = match options.auth_material.clone() {
        Some(auth_material) => ServerLaunchOptions::native_loopback_with_auth_material(
            options.port,
            options.session_bound,
            auth_material,
        ),
        None => ServerLaunchOptions::native_loopback(options.port, options.session_bound),
    };
    launch.with_repo_creation_projection_base(Some(options.app_data_dir.join("workspace")))
}

fn native_ai_provider_settings_root(app_data_dir: &Path, is_android: bool) -> PathBuf {
    if is_android {
        app_data_dir.join("files")
    } else {
        app_data_dir.to_path_buf()
    }
}

pub fn bind_native_loopback_listener(
    preferred_port: Option<u16>,
) -> std::io::Result<NativeLoopbackListener> {
    let preferred_port = preferred_port.unwrap_or(0);
    match bind_loopback_listener_on_port(preferred_port) {
        Ok(listener) => Ok(listener),
        Err(error) if preferred_port != 0 && error.kind() == std::io::ErrorKind::AddrInUse => {
            bind_loopback_listener_on_port(0)
        }
        Err(error) => Err(error),
    }
}

pub fn bind_native_loopback_listener_exact(port: u16) -> std::io::Result<NativeLoopbackListener> {
    bind_loopback_listener_on_port(port)
}

fn bind_loopback_listener_on_port(port: u16) -> std::io::Result<NativeLoopbackListener> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let port = listener.local_addr()?.port();
    Ok(NativeLoopbackListener { listener, port })
}

pub fn load_native_plugins(
    app_data_dir: &Path,
    native_ai_enabled: bool,
) -> anyhow::Result<Vec<Box<dyn PluginRuntime>>> {
    let plugin_dir = resolve_native_plugin_dir(app_data_dir)?;
    let loader = PluginLoader::new(plugin_dir.clone());
    let external_plugins = loader.load_all_strict()?;
    let plugins = crate::server::ai_chat::assemble_runtime_plugins_with_policy(
        external_plugins,
        native_ai_enabled,
    )?;
    tracing::info!(
        ?plugin_dir,
        "Registered {} native plugin runtimes.",
        plugins.len()
    );
    Ok(plugins)
}

fn resolve_native_plugin_dir(app_data_dir: &Path) -> anyhow::Result<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(value) = std::env::var_os(DEVE_PLUGIN_DIR_ENV).filter(|value| !value.is_empty()) {
        candidates.push(PathBuf::from(value));
    }
    candidates.extend(default_native_plugin_dir_candidates(app_data_dir));

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

    Ok(app_data_dir.join("plugins"))
}

fn default_native_plugin_dir_candidates(app_data_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![app_data_dir.join("plugins")];
    if let Some(exe_parent) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
    {
        candidates.push(exe_parent.join("plugins"));
        candidates.push(exe_parent.join("..").join("..").join("plugins"));
    }
    candidates
}

#[cfg(test)]
#[path = "native_runtime/tests.rs"]
mod tests;
