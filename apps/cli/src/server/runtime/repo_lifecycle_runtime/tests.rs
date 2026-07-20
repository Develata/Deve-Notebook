//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 03_storage/watcher#watcher-contract

use super::{CreateRepoIntent, RepoMountOutcome};
use crate::server::switcher_test_support::build_state;
use deve_core::ledger::RepoCatalogMembershipState;
use deve_core::models::RepoId;

async fn create_repo(
    state: &std::sync::Arc<crate::server::AppState>,
    projection_base: &std::path::Path,
    repo_id: RepoId,
) -> anyhow::Result<RepoMountOutcome> {
    let outcome = state
        .repo_lifecycle_coordinator()
        .create(CreateRepoIntent {
            repo_id,
            initial_alias: "created locally".to_string(),
            projection_base: projection_base.to_path_buf(),
            lifecycle_request_id: uuid::Uuid::new_v4(),
        })
        .await
        .map_err(anyhow::Error::from)?;
    Ok(outcome.mount)
}

#[tokio::test]
async fn create_mount_failure_keeps_cataloged_repo_readonly() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    let coordinator = state.repo_lifecycle_coordinator();
    let repo_id = RepoId::new_v4();
    coordinator.fail_next_watcher_start_for_test();

    let mount = create_repo(&state, &dir.path().join("notes"), repo_id).await?;

    assert_eq!(mount, RepoMountOutcome::Failed);
    assert!(
        state
            .repo
            .repo_catalog_membership_record(repo_id)?
            .is_some()
    );
    assert_eq!(
        state.repo.host_repo_alias_runtime().binding(repo_id)?.alias,
        "created locally"
    );
    assert!(!coordinator.watcher_is_mounted_for_test(repo_id));
    Ok(())
}

#[tokio::test]
async fn remove_stop_cleanup_failure_does_not_cut_catalog_membership() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    let coordinator = state.repo_lifecycle_coordinator();
    let fallback_id = RepoId::new_v4();
    let repo_id = RepoId::new_v4();
    assert_eq!(
        create_repo(&state, &dir.path().join("notes"), fallback_id).await?,
        RepoMountOutcome::Mounted
    );
    assert_eq!(
        create_repo(&state, &dir.path().join("notes"), repo_id).await?,
        RepoMountOutcome::Mounted
    );
    coordinator.fail_next_watcher_shutdown_after_cleanup_for_test();

    let removal = coordinator.remove(repo_id, uuid::Uuid::new_v4()).await;

    assert!(removal.is_err());
    assert!(
        state
            .repo
            .repo_catalog_membership_record(repo_id)?
            .is_some()
    );
    assert!(coordinator.watcher_is_mounted_for_test(repo_id));
    Ok(())
}

#[tokio::test]
async fn remove_publication_revalidates_fallback_mount_after_cut() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    let coordinator = state.repo_lifecycle_coordinator();
    let fallback_id = RepoId::new_v4();
    let repo_id = RepoId::new_v4();
    assert_eq!(
        create_repo(&state, &dir.path().join("notes"), fallback_id).await?,
        RepoMountOutcome::Mounted
    );
    assert_eq!(
        create_repo(&state, &dir.path().join("notes"), repo_id).await?,
        RepoMountOutcome::Mounted
    );
    coordinator.fail_fallback_before_publication_for_test();

    let outcome = coordinator.remove(repo_id, uuid::Uuid::new_v4()).await?;
    let fallback = outcome
        .fallback
        .expect("remove must choose a fallback repo");

    assert!(
        fallback
            .revalidate(&state.repo, &state.catalog_membership_runtime())
            .is_err(),
        "publication must not accept a fallback whose watcher failed after the catalog cut"
    );
    assert_eq!(
        state
            .repo
            .repo_catalog_membership_record(repo_id)?
            .map(|record| record.state()),
        Some(RepoCatalogMembershipState::Removed)
    );
    Ok(())
}
