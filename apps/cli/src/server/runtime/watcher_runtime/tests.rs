//! plan_ref:
//!   - 03_storage/authority#repo-mutation-publication-gate
//!   - 03_storage/watcher#watcher-contract

use super::{
    RepoMountState, WatcherLifecycleError, WatcherRuntimeAggregateStatus, WatcherRuntimeView,
    WatcherSupervisor,
};
use deve_core::models::RepoId;
use deve_core::sync::watcher::{
    RepoWatcherStart, WatcherFailure, WatcherFailureKind, WatcherFailurePhase, WatcherRefresh,
    WatcherRefreshCallback, WatcherRefreshKind,
};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

mod isolation;

type WatcherFixture = (
    tempfile::TempDir,
    Arc<deve_core::ledger::RepoManager>,
    Arc<deve_core::sync::SyncManager>,
    String,
    RepoId,
);

fn fixture() -> anyhow::Result<WatcherFixture> {
    let dir = tempfile::tempdir()?;
    let projection_base = dir.path().join("notes");
    std::fs::create_dir_all(&projection_base)?;
    let cataloged = crate::test_support::init_cataloged_repo_with_url(
        &dir.path().join("ledger"),
        &projection_base,
        8,
        Some("urn:main".to_string()),
    )?;
    let repo = Arc::new(cataloged.repo);
    let sync = Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?);
    let info = repo
        .get_repo_info_for(None, Some(&cataloged.repo_id.to_string()))?
        .expect("main repo");
    Ok((dir, repo, sync, info.name, info.uuid))
}

fn no_op_publisher() -> WatcherRefreshCallback {
    Arc::new(|_| {})
}

fn recording_publisher() -> (WatcherRefreshCallback, Arc<Mutex<Vec<WatcherRefresh>>>) {
    let refreshes = Arc::new(Mutex::new(Vec::new()));
    let output = refreshes.clone();
    (
        Arc::new(move |refresh| output.lock().expect("refresh output").push(refresh)),
        refreshes,
    )
}

#[test]
fn mounted_repo_admission_revalidates_the_exact_slot() {
    let repo_id = RepoId::new_v4();
    let view = WatcherRuntimeView::with_state_for_test(repo_id, 7, RepoMountState::Mounted);
    let token = view.admit(repo_id).expect("mounted repo admission");

    view.set_state_for_test(repo_id, RepoMountState::Failed);

    assert!(token.revalidate().is_err());
    assert!(view.admit(repo_id).is_err());
}

#[test]
fn non_mounted_states_and_unknown_repos_fail_closed() {
    for state in [
        RepoMountState::Starting,
        RepoMountState::Transitioning,
        RepoMountState::Failed,
        RepoMountState::Stopped,
    ] {
        let repo_id = RepoId::new_v4();
        let view = WatcherRuntimeView::with_state_for_test(repo_id, 1, state);
        assert!(view.admit(repo_id).is_err(), "state {state:?}");
        assert!(view.admit(RepoId::new_v4()).is_err(), "unknown repo");
    }
}

#[test]
fn watcher_failure_is_repo_local_in_runtime_view() {
    let failed_repo = RepoId::new_v4();
    let mounted_repo = RepoId::new_v4();
    let view = WatcherRuntimeView::with_state_for_test(failed_repo, 1, RepoMountState::Failed);

    view.insert_state_for_test(mounted_repo, 1, RepoMountState::Mounted);

    assert!(view.admit(failed_repo).is_err());
    assert!(view.admit(mounted_repo).is_ok());
}

#[test]
fn watcher_health_aggregate_counts_only_expected_repos() {
    let expected_mounted = RepoId::new_v4();
    let expected_failed = RepoId::new_v4();
    let ignored_mounted = RepoId::new_v4();
    let view =
        WatcherRuntimeView::with_state_for_test(expected_mounted, 1, RepoMountState::Mounted);
    view.insert_state_for_test(expected_failed, 1, RepoMountState::Failed);
    view.insert_state_for_test(ignored_mounted, 1, RepoMountState::Mounted);

    let aggregate = view.aggregate(&HashSet::from([expected_mounted, expected_failed]));

    assert_eq!(aggregate.status, WatcherRuntimeAggregateStatus::Degraded);
    assert_eq!(aggregate.expected, 2);
    assert_eq!(aggregate.running, 1);
    assert_eq!(aggregate.unavailable, 1);
}

#[test]
fn watcher_health_aggregate_applies_status_priority() {
    let mounted = RepoId::new_v4();
    let transitioning = RepoId::new_v4();
    let failed = RepoId::new_v4();
    let view = WatcherRuntimeView::with_state_for_test(mounted, 1, RepoMountState::Mounted);
    view.insert_state_for_test(transitioning, 1, RepoMountState::Transitioning);

    let transitioning_health = view.aggregate(&HashSet::from([mounted, transitioning]));
    assert_eq!(
        transitioning_health.status,
        WatcherRuntimeAggregateStatus::Transitioning
    );
    assert_eq!(transitioning_health.unavailable, 1);

    view.insert_state_for_test(failed, 1, RepoMountState::Failed);
    let degraded_health = view.aggregate(&HashSet::from([mounted, transitioning, failed]));
    assert_eq!(
        degraded_health.status,
        WatcherRuntimeAggregateStatus::Degraded
    );

    let missing = RepoId::new_v4();
    let missing_health = view.aggregate(&HashSet::from([mounted, missing]));
    assert_eq!(
        missing_health.status,
        WatcherRuntimeAggregateStatus::Degraded
    );
}

#[test]
fn watcher_health_aggregate_reports_unknown_for_incomplete_view() {
    let repo_id = RepoId::new_v4();
    let view = WatcherRuntimeView::with_state_for_test(repo_id, 1, RepoMountState::Mounted);
    view.poison_slots_for_test();

    let aggregate = view.aggregate(&HashSet::from([repo_id]));

    assert_eq!(aggregate.status, WatcherRuntimeAggregateStatus::Unknown);
    assert_eq!(aggregate.expected, 1);
    assert_eq!(aggregate.running, 0);
    assert_eq!(aggregate.unavailable, 1);
}

#[test]
fn supervisor_owns_mount_until_explicit_shutdown() -> anyhow::Result<()> {
    let (_dir, _repo, sync, repo_name, repo_id) = fixture()?;
    let supervisor = WatcherSupervisor::start_all(
        vec![RepoWatcherStart::resolve(sync, &repo_name, 1)?],
        no_op_publisher(),
    )?;
    let view = supervisor.view();
    assert!(view.admit(repo_id).is_ok());

    supervisor.shutdown()?;

    assert!(view.admit(repo_id).is_err());
    Ok(())
}

#[test]
fn supervisor_duplicate_reservation_fails_before_attach() -> anyhow::Result<()> {
    let (_dir, repo, sync, repo_name, repo_id) = fixture()?;
    let root = repo.local_repo_workspace_root(&repo_name)?;
    let first = RepoWatcherStart::resolve(sync.clone(), &repo_name, 1)?;
    let second = RepoWatcherStart::resolve(sync, &repo_name, 2)?;
    if root.try_exists()? {
        std::fs::remove_dir_all(root)?;
    }

    let error = match WatcherSupervisor::start_all(vec![first, second], no_op_publisher()) {
        Ok(_) => panic!("duplicate reservation must fail"),
        Err(error) => error,
    };

    assert!(matches!(
        error.kind(),
        super::error::WatcherHostFatalKind::SupervisorInvariant
    ));
    assert_eq!(error.repo_id(), Some(repo_id));
    Ok(())
}

#[test]
fn empty_repo_set_is_a_healthy_no_scope_runtime() -> anyhow::Result<()> {
    let supervisor = WatcherSupervisor::start_all(Vec::new(), no_op_publisher())?;
    let aggregate = supervisor.view().aggregate(&HashSet::new());

    assert_eq!(aggregate.status, WatcherRuntimeAggregateStatus::Healthy);
    assert_eq!(aggregate.expected, 0);
    assert_eq!(aggregate.running, 0);
    assert_eq!(aggregate.unavailable, 0);

    supervisor.shutdown()?;
    Ok(())
}

#[test]
fn concurrent_transition_reservation_fails_with_typed_busy() -> anyhow::Result<()> {
    let (_dir, _repo, sync, repo_name, repo_id) = fixture()?;
    let supervisor = WatcherSupervisor::start_all(
        vec![RepoWatcherStart::resolve(sync, &repo_name, 1)?],
        no_op_publisher(),
    )?;

    let reservation = supervisor.reserve_existing(repo_id)?;
    let error = match supervisor.reserve_existing(repo_id) {
        Ok(_) => panic!("second lifecycle intent must not wait"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        WatcherLifecycleError::Busy {
            repo_id: busy_repo,
            generation: 2,
            state: RepoMountState::Transitioning,
        } if busy_repo == repo_id
    ));
    supervisor.shutdown_reserved(&reservation)?;
    supervisor.finalize_removed(reservation)?;
    supervisor.shutdown()?;
    Ok(())
}

#[test]
fn remount_generation_invalidates_old_admission_and_flushes_one_refresh() -> anyhow::Result<()> {
    let (_dir, _repo, sync, repo_name, repo_id) = fixture()?;
    let (publisher, refreshes) = recording_publisher();
    let supervisor = WatcherSupervisor::start_all(
        vec![RepoWatcherStart::resolve(sync.clone(), &repo_name, 1)?],
        publisher,
    )?;
    refreshes.lock().expect("refreshes").clear();
    let view = supervisor.view();
    let old_admission = view.admit(repo_id)?;

    let reservation = supervisor.reserve_existing(repo_id)?;
    assert_eq!(reservation.previous_generation(), Some(1));
    assert_eq!(reservation.generation(), 2);
    assert!(old_admission.revalidate().is_err());
    supervisor.shutdown_reserved(&reservation)?;
    supervisor.route_refresh_for_test(
        repo_id,
        reservation.generation(),
        WatcherRefresh::new(repo_id, "a.md", WatcherRefreshKind::Added, false),
    )?;
    supervisor.route_refresh_for_test(
        repo_id,
        reservation.generation(),
        WatcherRefresh::new(repo_id, "b.md", WatcherRefreshKind::Modified, true),
    )?;
    assert!(refreshes.lock().expect("refreshes").is_empty());

    supervisor.start_reserved(
        &reservation,
        crate::server::setup::file_watcher_start(sync, &repo_name, reservation.generation())?,
    )?;
    let snapshot = supervisor.finalize_mounted(&reservation)?;
    assert_eq!(snapshot.repo_id(), repo_id);
    assert_eq!(snapshot.generation(), 2);
    assert_eq!(snapshot.state(), RepoMountState::Mounted);
    assert_eq!(supervisor.mounted_generation(repo_id)?, 2);
    let refreshes_guard = refreshes.lock().expect("refreshes");
    assert_eq!(refreshes_guard.len(), 1);
    assert_eq!(
        refreshes_guard[0].kind(),
        WatcherRefreshKind::DirectoryChanged
    );
    assert!(refreshes_guard[0].has_conflict());
    drop(refreshes_guard);

    supervisor.route_refresh_for_test(
        repo_id,
        2,
        WatcherRefresh::new(repo_id, "live.md", WatcherRefreshKind::Added, false),
    )?;
    assert_eq!(refreshes.lock().expect("refreshes").len(), 2);
    supervisor.shutdown()?;
    Ok(())
}

#[test]
fn removed_and_failed_slots_drop_generation_bound_refresh() -> anyhow::Result<()> {
    let (_dir, _repo, sync, repo_name, repo_id) = fixture()?;
    let (publisher, refreshes) = recording_publisher();
    let supervisor = WatcherSupervisor::start_all(
        vec![RepoWatcherStart::resolve(sync, &repo_name, 1)?],
        publisher,
    )?;
    refreshes.lock().expect("refreshes").clear();
    let reservation = supervisor.reserve_existing(repo_id)?;
    supervisor.shutdown_reserved(&reservation)?;
    supervisor.route_refresh_for_test(
        repo_id,
        reservation.generation(),
        WatcherRefresh::new(repo_id, "late.md", WatcherRefreshKind::Modified, false),
    )?;
    supervisor.discard_deferred(&reservation)?;
    supervisor.finalize_removed(reservation)?;
    assert!(refreshes.lock().expect("refreshes").is_empty());
    assert!(matches!(
        supervisor.snapshot(repo_id),
        Err(WatcherLifecycleError::Missing(id)) if id == repo_id
    ));

    let new_repo = RepoId::new_v4();
    let failed = supervisor.reserve_new(new_repo)?;
    supervisor.fail_for_test(
        new_repo,
        failed.generation(),
        WatcherFailure::new(
            WatcherFailurePhase::Worker,
            WatcherFailureKind::Backend,
            "injected terminal failure",
        ),
    )?;
    let snapshot = supervisor.snapshot(new_repo)?;
    assert_eq!(snapshot.state(), RepoMountState::Failed);
    assert!(snapshot.failure().is_some());
    supervisor.finalize_removed(failed)?;
    supervisor.shutdown()?;
    Ok(())
}

#[test]
fn supervisor_can_be_arc_shared_without_cloning_handle_ownership() -> anyhow::Result<()> {
    let (_dir, _repo, sync, repo_name, repo_id) = fixture()?;
    let supervisor = Arc::new(WatcherSupervisor::start_all(
        vec![RepoWatcherStart::resolve(sync, &repo_name, 1)?],
        no_op_publisher(),
    )?);
    let observer = supervisor.clone();

    assert_eq!(observer.snapshot(repo_id)?.generation(), 1);
    supervisor.shutdown()?;
    assert!(observer.view().admit(repo_id).is_err());
    Ok(())
}
