use super::build_sync_engine;
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use std::sync::Arc;

#[test]
fn server_sync_engine_uses_configured_sync_mode() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 8, Some("notes"), Some("urn:test:notes"))?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;

    let engine = build_sync_engine(PeerId::new("local"), Arc::new(repo), SyncMode::Manual);

    assert_eq!(engine.sync_mode(), SyncMode::Manual);
    Ok(())
}
