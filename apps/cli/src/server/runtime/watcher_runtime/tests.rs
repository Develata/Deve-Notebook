//! plan_ref:
//!   - 03_storage/authority#repo-mutation-publication-gate
//!   - 03_storage/watcher#watcher-contract

use super::{RepoMountState, WatcherRuntimeAggregateStatus, WatcherRuntimeView, WatcherSupervisor};
use deve_core::models::RepoId;
use deve_core::sync::watcher::RepoWatcherStart;
use std::collections::HashSet;
use std::sync::Arc;

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
    let repo = deve_core::ledger::RepoManager::init(
        dir.path().join("ledger"),
        8,
        Some("main"),
        Some("urn:main"),
    )?;
    repo.set_projection_base_for_local_repo("main", &projection_base)?;
    let repo = Arc::new(repo);
    let sync = Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?);
    let info = repo
        .get_repo_info_for(None, Some("main"))?
        .expect("main repo");
    Ok((dir, repo, sync, info.name, info.uuid))
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
    let supervisor =
        WatcherSupervisor::start_all(vec![RepoWatcherStart::resolve(sync, &repo_name, 1)?])?;
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

    let error = match WatcherSupervisor::start_all(vec![first, second]) {
        Ok(_) => panic!("duplicate reservation must fail"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        super::supervisor::WatcherSupervisorStartError::DuplicateRepo(id) if id == repo_id
    ));
    Ok(())
}
