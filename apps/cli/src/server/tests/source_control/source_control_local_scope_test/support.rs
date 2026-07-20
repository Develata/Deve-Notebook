//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use crate::server::{security, tree_state::RepoTreeRegistry, AppState};
use deve_core::config::SyncMode;
use deve_core::models::PeerId;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use tokio::sync::broadcast;

pub(super) fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>)> {
    let dir = tempdir()?;
    let projection_base = dir.path().join("notes");
    let repo =
        crate::test_support::init_cataloged_repo(&dir.path().join("ledger"), &projection_base, 10)?
            .repo;
    let repo = Arc::new(repo);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?),
            tx: broadcast::channel(16).0,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                PeerId::new("test-peer"),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_available: false,
            identity_key,
        }),
    ))
}

pub(super) fn write_workspace_file(dir: &TempDir, path: &str, content: &str) {
    let abs = default_workspace_root(dir).join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

pub(super) fn default_workspace_root(dir: &TempDir) -> std::path::PathBuf {
    let base = dir.path().join("notes");
    let content = std::fs::read_to_string(dir.path().join("ledger/.host/projection-locators.toml"))
        .expect("projection locator file");
    let value: toml::Value = toml::from_str(&content).expect("projection locator toml");
    let locator = value["locators"]
        .as_array()
        .expect("projection locators")
        .first()
        .expect("default repo locator");
    base.join(
        locator["workspace_segment"]
            .as_str()
            .expect("workspace segment"),
    )
}
