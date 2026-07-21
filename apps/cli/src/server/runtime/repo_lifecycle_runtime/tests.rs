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

#[tokio::test]
async fn remove_last_repo_enters_no_scope_without_requiring_fallback() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let coordinator = state.repo_lifecycle_coordinator();
    let only_repo = state
        .repo
        .list_cataloged_local_repo_summaries()?
        .into_iter()
        .next()
        .expect("fixture repo");

    let outcome = coordinator
        .remove(only_repo.repo_id, uuid::Uuid::new_v4())
        .await?;

    assert!(outcome.fallback.is_none());
    assert!(state.repo.list_cataloged_local_repo_summaries()?.is_empty());
    assert!(!coordinator.watcher_is_mounted_for_test(only_repo.repo_id));
    coordinator.shutdown_watchers_for_test();
    Ok(())
}

#[tokio::test]
async fn remove_last_then_create_uses_new_durable_membership_for_default_reads()
-> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    let coordinator = state.repo_lifecycle_coordinator();
    let old_repo = state
        .repo
        .list_cataloged_local_repo_summaries()?
        .into_iter()
        .next()
        .expect("fixture repo");
    coordinator
        .remove(old_repo.repo_id, uuid::Uuid::new_v4())
        .await?;

    let new_repo = RepoId::new_v4();
    assert_eq!(
        create_repo(&state, &dir.path().join("replacement-notes"), new_repo).await?,
        RepoMountOutcome::Mounted
    );

    assert_eq!(state.repo.current_local_repo_name()?, new_repo.to_string());
    assert!(state.repo.list_local_docs(None)?.is_empty());
    coordinator.shutdown_watchers_for_test();
    Ok(())
}

#[tokio::test]
async fn queued_create_rejects_removed_projection_base_source() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let coordinator = state.repo_lifecycle_coordinator();
    let source = state
        .repo
        .list_cataloged_local_repo_summaries()?
        .into_iter()
        .next()
        .expect("fixture repo");
    let prepared_base = state
        .repo
        .projection_locator_for_local_repo(&source.execution_name)?
        .projection_base_abs;

    coordinator
        .remove(source.repo_id, uuid::Uuid::new_v4())
        .await?;

    let error = coordinator
        .revalidate_create_projection_base(Some(source.repo_id), &prepared_base)
        .expect_err("removed source repo must invalidate a queued create");
    assert!(error.to_string().contains("left the local catalog"));
    coordinator.shutdown_watchers_for_test();
    Ok(())
}

#[test]
fn queued_create_rejects_projection_base_locator_drift() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    let coordinator = state.repo_lifecycle_coordinator();
    let source = state
        .repo
        .list_cataloged_local_repo_summaries()?
        .into_iter()
        .next()
        .expect("fixture repo");
    let prepared_base = state
        .repo
        .projection_locator_for_local_repo(&source.execution_name)?
        .projection_base_abs;
    let relocated = dir.path().join("relocated-notes");
    std::fs::create_dir_all(&relocated)?;
    state
        .repo
        .set_projection_base_for_repo_id(source.repo_id, &relocated)?;

    let error = coordinator
        .revalidate_create_projection_base(Some(source.repo_id), &prepared_base)
        .expect_err("locator drift must invalidate a queued create");
    assert!(error.to_string().contains("binding changed"));
    coordinator.shutdown_watchers_for_test();
    Ok(())
}
