use super::{ensure_source_control_write_allowed, ensure_source_control_write_allowed_for};
use crate::ledger::RepoManager;
use crate::ledger::traits::RepoSelector;
use crate::protocol::ServerErrorCode;
use crate::sync::SyncManager;
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn source_control_write_gate_allows_proxy_without_local_projection_state() {
    ensure_source_control_write_allowed(&RepoSelector::default())
        .expect("plugin-host proxy should delegate write checks to the main process");
}

#[test]
fn source_control_write_gate_rejects_degraded_local_projection() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_all_local_repos(&vault);
    let repo = Arc::new(repo);
    let sync = SyncManager::new(repo.clone());
    sync.mark_projection_writeback_fault("default");

    let error =
        ensure_source_control_write_allowed_for(repo.as_ref(), &sync, &RepoSelector::default())
            .expect_err("degraded projection must reject plugin source-control writes");

    assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
    assert!(
        error
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("projection is degraded"))
    );
    Ok(())
}

#[test]
fn source_control_write_gate_accepts_healthy_local_projection() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_all_local_repos(&vault);
    let repo = Arc::new(repo);
    let sync = SyncManager::new(repo.clone());

    ensure_source_control_write_allowed_for(repo.as_ref(), &sync, &RepoSelector::default())
        .expect("healthy projection should allow plugin source-control writes");
    Ok(())
}
