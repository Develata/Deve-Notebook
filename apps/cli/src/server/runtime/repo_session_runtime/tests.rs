//! plan_ref:
//!   - 04_repository#repo-health-and-repair
//!   - 09_web_thin_client_ledger#write-readiness
//!   - 07_network#server-ws-runtime

use super::*;
#[test]
fn permit_drop_removes_registration() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo =
        deve_core::ledger::RepoManager::init(dir.path(), 4, Some("default"), None).expect("repo");
    let membership = repo.catalog_membership_runtime();
    repo.seed_catalog_membership_from_records()
        .expect("seed membership runtime");
    let runtime = RepoSessionRuntime::new(membership);
    let (broadcast, _) = tokio::sync::broadcast::channel(2);
    let (unicast, _) = tokio::sync::mpsc::channel(2);
    let (permit, _commands) = runtime
        .register(DualChannel::new(broadcast, unicast))
        .expect("register");
    assert_eq!(runtime.registered_sessions(), 1);
    drop(permit);
    assert_eq!(runtime.registered_sessions(), 0);
}

struct BoundSessionFixture {
    _dir: tempfile::TempDir,
    runtime: Arc<RepoSessionRuntime>,
    repo_id: RepoId,
    token: CatalogMembershipToken,
}

fn bound_session_fixture() -> BoundSessionFixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = crate::test_support::init_cataloged_repo(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        4,
    )
    .expect("cataloged repo");
    let membership = fixture.repo.catalog_membership_runtime();
    let token = membership.issue(fixture.repo_id).expect("membership token");
    BoundSessionFixture {
        _dir: dir,
        runtime: RepoSessionRuntime::new(membership),
        repo_id: fixture.repo_id,
        token,
    }
}

fn register_session(
    runtime: &Arc<RepoSessionRuntime>,
) -> (
    RepoSessionPermit,
    mpsc::Receiver<RepoSessionCommand>,
    DualChannel,
    tokio::sync::watch::Receiver<bool>,
) {
    let (broadcast, _) = tokio::sync::broadcast::channel(2);
    let (unicast, _) = tokio::sync::mpsc::channel(2);
    let channel = DualChannel::new(broadcast, unicast);
    let retirement = channel.retirement_receiver();
    let (permit, commands) = runtime.register(channel.clone()).expect("register");
    (permit, commands, channel, retirement)
}

fn bound_ws_session(repo_id: RepoId, token: &CatalogMembershipToken) -> WsSession {
    let mut session = WsSession::new();
    session.browser_session = true;
    session.active_repo_id = Some(repo_id);
    session.catalog_membership = Some(token.clone());
    session.current_scope_nonce = 7;
    session
}

#[test]
fn lifecycle_observer_registration_fails_closed_on_missing_session() {
    let fixture = bound_session_fixture();
    let request_id = uuid::Uuid::new_v4();
    assert_eq!(
        fixture
            .runtime
            .register_lifecycle_observer(999, request_id, 1, 2),
        Err(RepoSessionRuntimeError::SessionMissing)
    );
    assert_eq!(
        fixture.runtime.clear_lifecycle_observer(999, request_id),
        Err(RepoSessionRuntimeError::SessionMissing)
    );
}

#[test]
fn publish_lifecycle_settlement_targets_exact_observer_then_none() {
    let fixture = bound_session_fixture();
    let (permit, mut commands, _channel, _retirement) = register_session(&fixture.runtime);
    let request_id = uuid::Uuid::new_v4();
    let job_id = uuid::Uuid::new_v4();
    fixture
        .runtime
        .register_lifecycle_observer(permit.id(), request_id, 7, 8)
        .expect("register observer");

    let publication =
        super::super::repo_lifecycle_job_runtime::RepoLifecycleSettledPublication::Removed {
            repo_id: fixture.repo_id,
            fallback_repo_id: None,
        };
    let delivered = fixture
        .runtime
        .publish_lifecycle_settlement(
            request_id,
            job_id,
            publication,
            FinalRepoListProjection { entries: vec![] },
        )
        .expect("publish");
    assert_eq!(delivered, Some(permit.id()));
    match commands.try_recv().expect("queued settlement command") {
        RepoSessionCommand::LifecycleSettled {
            request_id: seen,
            job_id: seen_job_id,
            expected_scope_nonce,
            switch_nonce,
            ..
        } => {
            assert_eq!(seen, request_id);
            assert_eq!(seen_job_id, job_id);
            assert_eq!(expected_scope_nonce, 7);
            assert_eq!(switch_nonce, 8);
        }
        RepoSessionCommand::Removed { .. } => panic!("expected settlement command"),
    }

    // The observer is consumed by delivery; replay needs re-registration.
    let publication =
        super::super::repo_lifecycle_job_runtime::RepoLifecycleSettledPublication::Removed {
            repo_id: fixture.repo_id,
            fallback_repo_id: None,
        };
    let redelivered = fixture
        .runtime
        .publish_lifecycle_settlement(
            request_id,
            job_id,
            publication,
            FinalRepoListProjection { entries: vec![] },
        )
        .expect("publish without observer");
    assert_eq!(redelivered, None);
}

#[test]
fn invalidate_removed_repo_observers_fans_out_once_and_excludes_initiator() {
    let fixture = bound_session_fixture();
    let (initiator, _initiator_commands, _ic, _ir) = register_session(&fixture.runtime);
    let (observer, mut observer_commands, _oc, _or) = register_session(&fixture.runtime);
    let session = bound_ws_session(fixture.repo_id, &fixture.token);
    initiator.update(&session).expect("bind initiator");
    observer.update(&session).expect("bind observer");

    let invalidated = fixture
        .runtime
        .invalidate_removed_repo_observers(
            uuid::Uuid::new_v4(),
            fixture.repo_id,
            Some(initiator.id()),
            FinalRepoListProjection { entries: vec![] },
        )
        .expect("invalidate");
    assert_eq!(invalidated, 1);
    match observer_commands
        .try_recv()
        .expect("queued removal command")
    {
        RepoSessionCommand::Removed {
            removed_repo_id,
            expected_scope_nonce,
            next_scope_nonce,
            ..
        } => {
            assert_eq!(removed_repo_id, fixture.repo_id);
            assert_eq!(expected_scope_nonce, 7);
            assert_eq!(next_scope_nonce, 8);
        }
        RepoSessionCommand::LifecycleSettled { .. } => panic!("expected removal command"),
    }

    // Delivered bindings are consumed; the excluded initiator keeps its
    // binding (its own settlement travels via LifecycleSettled), so an
    // unexcluded second cut reaches exactly the initiator, and a third
    // finds nothing.
    let invalidated = fixture
        .runtime
        .invalidate_removed_repo_observers(
            uuid::Uuid::new_v4(),
            fixture.repo_id,
            None,
            FinalRepoListProjection { entries: vec![] },
        )
        .expect("second invalidate");
    assert_eq!(invalidated, 1);
    let invalidated = fixture
        .runtime
        .invalidate_removed_repo_observers(
            uuid::Uuid::new_v4(),
            fixture.repo_id,
            None,
            FinalRepoListProjection { entries: vec![] },
        )
        .expect("third invalidate");
    assert_eq!(invalidated, 0);
}

#[test]
fn invalidate_removed_repo_observers_retires_session_when_commands_undeliverable() {
    let fixture = bound_session_fixture();
    let (observer, observer_commands, _channel, mut retirement) =
        register_session(&fixture.runtime);
    let session = bound_ws_session(fixture.repo_id, &fixture.token);
    observer.update(&session).expect("bind observer");
    drop(observer_commands);

    let invalidated = fixture
        .runtime
        .invalidate_removed_repo_observers(
            uuid::Uuid::new_v4(),
            fixture.repo_id,
            None,
            FinalRepoListProjection { entries: vec![] },
        )
        .expect("invalidate with closed channel");
    assert_eq!(invalidated, 0, "undeliverable command must not count");
    assert!(
        *retirement.borrow_and_update(),
        "session with undeliverable command must be retired"
    );
}
