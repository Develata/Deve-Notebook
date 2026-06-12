use super::{ensure_source_control_write_allowed, ensure_source_control_write_allowed_for};
use crate::ledger::RepoManager;
use crate::ledger::traits::RepoSelector;
use crate::protocol::ServerErrorCode;
use crate::sync::SyncManager;
use crate::utils::notegit::repo_identity_path;
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn source_control_write_gate_missing_dependencies_fail_closed() {
    let error = ensure_source_control_write_allowed(&RepoSelector::default())
        .expect_err("plugin source-control writes must fail closed without host setup");

    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert!(
        error
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("write gate"))
    );
}

#[test]
fn source_control_write_gate_rejects_degraded_local_projection() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 10, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let repo = Arc::new(repo);
    let sync = SyncManager::new_checked(repo.clone())?;
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
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 10, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    repo.ensure_local_repo_workspace_identity("default")?;
    let repo = Arc::new(repo);
    let sync = SyncManager::new_checked(repo.clone())?;

    ensure_source_control_write_allowed_for(repo.as_ref(), &sync, &RepoSelector::default())
        .expect("healthy projection should allow plugin source-control writes");
    Ok(())
}

#[test]
fn source_control_write_gate_rejects_broken_workspace_identity() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 10, Some("default"), Some("urn:default"))?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let workspace = repo.ensure_local_repo_workspace_identity("default")?;
    std::fs::write(
        repo_identity_path(&workspace),
        format!(
            "version = 1\nrepo_id = \"{}\"\nrepo_name = \"default\"\n",
            uuid::Uuid::new_v4()
        ),
    )?;
    let repo = Arc::new(repo);
    let sync = SyncManager::new_checked(repo.clone())?;

    let error =
        ensure_source_control_write_allowed_for(repo.as_ref(), &sync, &RepoSelector::default())
            .expect_err("broken workspace identity must reject plugin source-control writes");

    assert_eq!(error.code, ServerErrorCode::StoragePersistFailed);
    assert!(
        error
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("identity marker"))
    );
    Ok(())
}
