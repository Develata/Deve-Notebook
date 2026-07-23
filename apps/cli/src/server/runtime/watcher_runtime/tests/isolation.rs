//! plan_ref:
//!   - 03_storage/authority#repo-mutation-publication-gate
//!   - 03_storage/watcher#watcher-contract

use super::super::error::WatcherHostFatalKind;
use super::super::slot::MountSlot;
use super::super::{
    RepoMountState, WatcherRuntimeAggregateStatus, WatcherSupervisor, start_file_watchers,
};
use super::{fixture, no_op_publisher};
use deve_core::models::RepoId;
use deve_core::sync::watcher::{
    RepoWatcherStart, WatcherFailure, WatcherFailureKind, WatcherFailurePhase, WatcherRefresh,
    WatcherRefreshCallback, WatcherRefreshKind,
};
use std::collections::HashSet;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

fn add_cataloged_repo(
    repo: &deve_core::ledger::RepoManager,
    projection_base: &std::path::Path,
) -> anyhow::Result<(RepoId, std::path::PathBuf)> {
    let repo_id = RepoId::new_v4();
    let execution_name = repo_id.to_string();
    let (_info, prepared_authority) = repo.create_local_repo_authority(repo_id, None)?;
    let locator = repo.prepare_projection_locator_for_repo_creation_with_authority(
        repo_id,
        projection_base,
        &prepared_authority,
    )?;
    let workspace = locator.projection_base_abs.join(&locator.workspace_segment);
    std::fs::create_dir_all(&workspace)?;
    deve_core::utils::notegit::ensure_repo_identity_marker(&workspace, repo_id, &execution_name)?;
    let authority = repo.claim_repo_catalog_cut_authority()?;
    let prepared = repo.prepare_repo_creation_membership_with_authority(
        repo_id,
        uuid::Uuid::new_v4(),
        &prepared_authority,
    )?;
    let revalidated =
        repo.revalidate_repo_creation_membership_with_authority(&prepared, &prepared_authority)?;
    let permit = authority.permit(repo_id)?;
    let commit = repo.commit_repo_creation_membership(&prepared, &revalidated, &permit)?;
    repo.activate_prepared_local_repo_authority(prepared_authority, &prepared, &commit)?;
    Ok((repo_id, workspace))
}

#[test]
fn watcher_server_isolation_bootstrap_keeps_healthy_repo_mounted() -> anyhow::Result<()> {
    let (_dir, _repo, sync, repo_name, mounted_repo) = fixture()?;
    let failed_repo = RepoId::new_v4();
    let supervisor = WatcherSupervisor::start_all(
        vec![
            RepoWatcherStart::resolve(sync.clone(), &repo_name, 1)?,
            RepoWatcherStart::new(sync, failed_repo, &repo_name, 1),
        ],
        no_op_publisher(),
    )?;
    let view = supervisor.view();

    assert!(view.admit(mounted_repo).is_ok());
    assert!(view.admit(failed_repo).is_err());
    let aggregate = view.aggregate(&HashSet::from([mounted_repo, failed_repo]));
    assert_eq!(aggregate.status, WatcherRuntimeAggregateStatus::Degraded);
    assert_eq!(aggregate.running, 1);
    assert_eq!(aggregate.unavailable, 1);

    supervisor.fail_for_test(
        mounted_repo,
        1,
        WatcherFailure::new(
            WatcherFailurePhase::Worker,
            WatcherFailureKind::Backend,
            "injected runtime terminal failure",
        ),
    )?;
    let all_failed = view.aggregate(&HashSet::from([mounted_repo, failed_repo]));
    assert_eq!(all_failed.status, WatcherRuntimeAggregateStatus::Degraded);
    assert_eq!(all_failed.running, 0);
    assert_eq!(all_failed.unavailable, 2);
    assert!(view.admit(mounted_repo).is_err());

    supervisor.shutdown()?;
    Ok(())
}

#[test]
fn watcher_server_isolation_production_bootstrap_contains_repo_local_prepare_failure()
-> anyhow::Result<()> {
    let (dir, repo, sync, _repo_name, mounted_repo) = fixture()?;
    let projection_base = std::fs::canonicalize(dir.path().join("notes"))?;
    let (failed_repo, failed_workspace) = add_cataloged_repo(&repo, &projection_base)?;
    std::fs::remove_dir_all(&failed_workspace)?;
    std::fs::write(&failed_workspace, b"not a directory")?;
    let (tx, _rx) = tokio::sync::broadcast::channel(8);

    let supervisor = start_file_watchers(sync, tx)?;
    let view = supervisor.view();

    assert!(view.admit(mounted_repo).is_ok());
    assert!(view.admit(failed_repo).is_err());
    let aggregate = view.aggregate(&HashSet::from([mounted_repo, failed_repo]));
    assert_eq!(aggregate.status, WatcherRuntimeAggregateStatus::Degraded);
    assert_eq!(aggregate.running, 1);
    assert_eq!(aggregate.unavailable, 1);

    supervisor.shutdown()?;
    Ok(())
}

#[test]
fn watcher_server_isolation_zero_mounted_keeps_supervisor_available() -> anyhow::Result<()> {
    let (_dir, _repo, sync, repo_name, _cataloged_repo) = fixture()?;
    let failed_repo = RepoId::new_v4();
    let supervisor = WatcherSupervisor::start_all(
        vec![RepoWatcherStart::new(sync, failed_repo, &repo_name, 1)],
        no_op_publisher(),
    )?;
    let view = supervisor.view();

    assert!(view.admit(failed_repo).is_err());
    let aggregate = view.aggregate(&HashSet::from([failed_repo]));
    assert_eq!(aggregate.status, WatcherRuntimeAggregateStatus::Degraded);
    assert_eq!(aggregate.running, 0);
    assert_eq!(aggregate.unavailable, 1);

    supervisor.shutdown()?;
    Ok(())
}

#[test]
fn watcher_server_isolation_typed_host_fatal_rolls_back_all_started_handles() -> anyhow::Result<()>
{
    let (dir, repo, sync, repo_name, repo_id) = fixture()?;
    let projection_base = std::fs::canonicalize(dir.path().join("notes"))?;
    let (second_repo, _) = add_cataloged_repo(&repo, &projection_base)?;
    let observed = Arc::new(Mutex::new(None));
    let error = match WatcherSupervisor::start_all_with_host_fatal_before_for_test(
        vec![
            RepoWatcherStart::resolve(sync.clone(), &repo_name, 1)?,
            RepoWatcherStart::new(sync, second_repo, second_repo.to_string(), 1),
        ],
        no_op_publisher(),
        1,
        observed.clone(),
    ) {
        Ok(_) => panic!("typed host-fatal must abort watcher bootstrap"),
        Err(error) => error,
    };

    assert_eq!(error.kind(), WatcherHostFatalKind::ThreadResourceExhaustion);
    let view = observed
        .lock()
        .expect("watcher bootstrap observer")
        .take()
        .expect("view captured after first watcher mounted");
    assert!(view.admit(repo_id).is_err());
    assert!(view.admit(second_repo).is_err());
    Ok(())
}

#[test]
fn watcher_server_isolation_cancel_preserves_terminal_failure_cut() -> anyhow::Result<()> {
    let (_dir, _repo, sync, repo_name, repo_id) = fixture()?;
    let supervisor = WatcherSupervisor::start_all(
        vec![RepoWatcherStart::resolve(sync, &repo_name, 1)?],
        no_op_publisher(),
    )?;
    let view = supervisor.view();
    let reservation = supervisor.reserve_existing(repo_id)?;
    let previous = reservation
        .previous
        .as_ref()
        .expect("existing mount reservation")
        .clone();
    previous.fail(WatcherFailure::new(
        WatcherFailurePhase::Worker,
        WatcherFailureKind::Backend,
        "injected terminal failure during lifecycle transition",
    ));

    supervisor.cancel_unstarted(reservation)?;

    let snapshot = supervisor.snapshot(repo_id)?;
    assert_eq!(snapshot.state(), RepoMountState::Failed);
    assert!(snapshot.failure().is_some());
    assert!(view.admit(repo_id).is_err());
    supervisor.shutdown()?;
    Ok(())
}

#[test]
fn watcher_server_isolation_failure_cut_does_not_wait_for_refresh_publication() -> anyhow::Result<()>
{
    let (_dir, _repo, sync, repo_name, repo_id) = fixture()?;
    let (entered_tx, entered_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let release_rx = Arc::new(Mutex::new(release_rx));
    let publisher: WatcherRefreshCallback = Arc::new(move |_| {
        let _ = entered_tx.send(());
        let _ = release_rx.lock().expect("publisher release").recv();
    });
    let supervisor = Arc::new(WatcherSupervisor::start_all(
        vec![RepoWatcherStart::resolve(sync, &repo_name, 1)?],
        publisher,
    )?);
    let route_supervisor = supervisor.clone();
    let route = std::thread::spawn(move || {
        route_supervisor.route_refresh_for_test(
            repo_id,
            1,
            WatcherRefresh::new(repo_id, "blocked.md", WatcherRefreshKind::Modified, false),
        )
    });
    entered_rx.recv_timeout(Duration::from_secs(1))?;

    let failure_supervisor = supervisor.clone();
    let (failed_tx, failed_rx) = mpsc::sync_channel(1);
    let failure = std::thread::spawn(move || {
        let result = failure_supervisor.fail_for_test(
            repo_id,
            1,
            WatcherFailure::new(
                WatcherFailurePhase::Worker,
                WatcherFailureKind::Backend,
                "injected failure while publisher is blocked",
            ),
        );
        let _ = failed_tx.send(result);
    });
    let failed_before_release = failed_rx.recv_timeout(Duration::from_millis(250));
    let _ = release_tx.send(());
    route.join().expect("refresh route thread")?;
    failure.join().expect("failure callback thread");

    failed_before_release
        .map_err(|_| anyhow::anyhow!("failure callback waited for refresh publication"))??;
    assert!(supervisor.view().admit(repo_id).is_err());
    supervisor.shutdown()?;
    Ok(())
}

#[test]
fn watcher_server_isolation_slot_snapshot_keeps_enriched_cleanup() {
    let repo_id = RepoId::new_v4();
    let slot = MountSlot::mounted(repo_id, 1);
    let primary = WatcherFailure::new(
        WatcherFailurePhase::Worker,
        WatcherFailureKind::Backend,
        "primary failure",
    );
    slot.fail(primary.clone());
    let mut enriched = primary;
    enriched.cleanup.push("shutdown cleanup".to_string());

    slot.mark_failed_and_drop(enriched);

    assert_eq!(
        slot.snapshot().failure().expect("slot failure").cleanup,
        vec!["shutdown cleanup".to_string()]
    );
}
