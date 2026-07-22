//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!   - 07_network#repo-control-wire-contract

use super::super::removal::{
    RepoRemovalExecuteIntent, RepoRemovalIssuerBinding, RepoRemovalPrepareIntent,
    RepoRemovalPrepared,
};
use super::super::{
    RepoLifecycleHostExecutor, RepoLifecycleHostPublicationSink, RepoLifecycleJobError,
    RepoLifecycleJobOutcome, RepoLifecycleJobPhase, RepoLifecycleJobRuntime,
};
use super::create_intent;
use crate::server::AppState;
use crate::server::switcher_test_support::{
    app_state as build_app_state, build_state as build_state_inner,
};
use anyhow::Context;
use deve_core::models::RepoId;
use deve_core::utils::fs::{HostPathIdentity, HostPathKind};
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

static REMOVAL_INTEGRATION_TEST_MUTEX: LazyLock<Arc<tokio::sync::Mutex<()>>> =
    LazyLock::new(|| Arc::new(tokio::sync::Mutex::new(())));

mod cut_recovery;
mod recovery;
mod success;

async fn build_state() -> anyhow::Result<(
    tokio::sync::OwnedMutexGuard<()>,
    tempfile::TempDir,
    Arc<AppState>,
)> {
    let guard = REMOVAL_INTEGRATION_TEST_MUTEX.clone().lock_owned().await;
    let (dir, state) = build_state_inner()?;
    Ok((guard, dir, state))
}

fn web_issuer(connection_epoch: u64) -> RepoRemovalIssuerBinding {
    web_issuer_with_principal('a', connection_epoch)
}

fn web_issuer_with_principal(principal: char, connection_epoch: u64) -> RepoRemovalIssuerBinding {
    RepoRemovalIssuerBinding::Web {
        principal_digest: principal.to_string().repeat(64),
        connection_epoch,
    }
}

fn offline_issuer(state: &AppState, repo_id: RepoId) -> anyhow::Result<RepoRemovalIssuerBinding> {
    let authority_root =
        HostPathIdentity::capture(state.repo.ledger_dir(), HostPathKind::Directory)?;
    let authority_lock = state
        .repo
        .snapshot_local_authority_for_removal(repo_id)?
        .authority_lock()
        .clone();
    Ok(RepoRemovalIssuerBinding::OfflineAuthority {
        authority_root,
        authority_lock,
    })
}

async fn prepare(
    runtime: &RepoLifecycleJobRuntime,
    repo_id: RepoId,
    request_id: Uuid,
    issuer: RepoRemovalIssuerBinding,
) -> Result<RepoRemovalPrepared, RepoLifecycleJobError> {
    runtime
        .prepare_removal(RepoRemovalPrepareIntent {
            request_id,
            repo_id,
            scope_nonce: 7,
            fallback_repo_id: None,
            issuer,
        })
        .await
}

fn execute_intent(
    prepared: &RepoRemovalPrepared,
    token: deve_core::protocol::RemovalConfirmationToken,
    issuer: RepoRemovalIssuerBinding,
) -> RepoRemovalExecuteIntent {
    RepoRemovalExecuteIntent {
        request_id: Uuid::new_v4(),
        expected_repo_id: None,
        preparation_id: prepared.preparation_id,
        confirmation_token: token,
        fallback_binding: prepared.fallback_binding.clone(),
        scope_nonce: 7,
        switch_nonce: 8,
        issuer,
    }
}

async fn terminal_status(
    runtime: &RepoLifecycleJobRuntime,
    request_id: Uuid,
) -> Result<super::super::RepoLifecycleJobStatus, RepoLifecycleJobError> {
    // The lifecycle worker may be sharing a single-job test host with the
    // full workspace suite. Keep this a hard producer timeout, but leave
    // enough headroom for a loaded Windows runner to reach the terminal cut.
    timeout(Duration::from_secs(10), async {
        loop {
            let status = runtime.status(request_id).await?;
            if status.phase == RepoLifecycleJobPhase::Terminal {
                break Ok(status);
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .map_err(|_| RepoLifecycleJobError::Coordination("test lifecycle status timeout"))?
}

fn restart_runtime(
    state: &Arc<AppState>,
) -> Result<Arc<RepoLifecycleJobRuntime>, RepoLifecycleJobError> {
    RepoLifecycleJobRuntime::start(
        state.repo.ledger_dir(),
        Arc::new(RepoLifecycleHostExecutor::new(
            state.repo_lifecycle_coordinator(),
            state.repo.clone(),
            state.watcher_runtime_view(),
            state.sync_manager.clone(),
            state.remote_import_coordinator(),
        )),
        Arc::new(RepoLifecycleHostPublicationSink::new(
            state.repo.clone(),
            state.watcher_runtime_view(),
            state.repo_session_runtime(),
            state.tx.clone(),
        )),
    )
}

fn rebuild_cold_host(dir: &tempfile::TempDir) -> anyhow::Result<Arc<AppState>> {
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let repo = deve_core::ledger::RepoManager::init_empty_host(&ledger_dir, 10)?;
    build_app_state(
        repo,
        projection_base,
        deve_core::utils::notegit::host_keys_dir(&ledger_dir),
    )
}

fn rebuild_cold_host_for_repo(
    dir: &tempfile::TempDir,
    repo_id: RepoId,
) -> anyhow::Result<Arc<AppState>> {
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let repo = deve_core::ledger::RepoManager::init_existing_for_repo_id(&ledger_dir, 10, repo_id)?;
    build_app_state(
        repo,
        projection_base,
        deve_core::utils::notegit::host_keys_dir(&ledger_dir),
    )
}

#[tokio::test]
async fn prepare_local_repo_removal_reissues_and_invalidates_confirmation_token()
-> anyhow::Result<()> {
    let (test_guard, dir, state) = build_state().await?;
    let repo_id = state.repo.list_cataloged_local_repo_summaries()?[0].repo_id;
    let runtime = state.repo_lifecycle_jobs();
    let request_id = Uuid::new_v4();
    let first = prepare(&runtime, repo_id, request_id, web_issuer(11)).await?;
    let first_token = first
        .confirmation_token
        .unwrap_or_else(|| panic!("first preview blocked: {:?}", first.preview));
    let second = prepare(&runtime, repo_id, request_id, web_issuer(11)).await?;
    let second_token = second
        .confirmation_token
        .clone()
        .expect("replacement token");
    assert_eq!(second.preparation_id, first.preparation_id);
    assert_ne!(first_token.as_str(), second_token.as_str());

    let stale = runtime
        .execute_removal(execute_intent(&second, first_token, web_issuer(11)))
        .await
        .expect_err("reissued token must invalidate the prior token");
    assert!(matches!(stale, RepoLifecycleJobError::ConfirmationInvalid));
    let record_path = dir
        .path()
        .join("ledger/.host/repo-lifecycle-jobs/removals")
        .join(format!("{}.json", second.preparation_id));
    assert!(!std::fs::read_to_string(record_path)?.contains(second_token.as_str()));
    runtime.shutdown().await?;
    drop((state, dir, test_guard));
    Ok(())
}

#[tokio::test]
async fn execute_local_repo_removal_rejects_expired_stale_and_wrong_issuer_token()
-> anyhow::Result<()> {
    let (test_guard, dir, state) = build_state().await?;
    let repo_id = state.repo.list_cataloged_local_repo_summaries()?[0].repo_id;
    let runtime = state.repo_lifecycle_jobs();
    let prepared = prepare(&runtime, repo_id, Uuid::new_v4(), web_issuer(20)).await?;
    let token = prepared
        .confirmation_token
        .clone()
        .unwrap_or_else(|| panic!("preview blocked: {:?}", prepared.preview));

    let wrong_issuer = runtime
        .execute_removal(execute_intent(&prepared, token.clone(), web_issuer(21)))
        .await
        .expect_err("connection epoch drift must invalidate confirmation");
    assert!(matches!(
        wrong_issuer,
        RepoLifecycleJobError::ConfirmationInvalid
    ));

    let expired = runtime
        .execute_removal_at_for_test(
            execute_intent(&prepared, token, web_issuer(20)),
            prepared.expires_at_unix_ms.expect("expiry"),
        )
        .await
        .expect_err("expired confirmation must fail closed");
    assert!(matches!(
        expired,
        RepoLifecycleJobError::ConfirmationExpired
    ));

    let prepared = prepare(&runtime, repo_id, Uuid::new_v4(), web_issuer(20)).await?;
    let alias = state.repo.host_repo_alias_runtime().binding(repo_id)?;
    state.repo.host_repo_alias_runtime().set_alias(
        repo_id,
        "alias changed after preview",
        alias.alias_revision,
    )?;
    let stale = runtime
        .execute_removal(execute_intent(
            &prepared,
            prepared
                .confirmation_token
                .clone()
                .unwrap_or_else(|| panic!("preview blocked: {:?}", prepared.preview)),
            web_issuer(20),
        ))
        .await
        .expect_err("manifest drift must invalidate confirmation");
    assert!(matches!(stale, RepoLifecycleJobError::ConfirmationStale));

    let prepared = prepare(&runtime, repo_id, Uuid::new_v4(), web_issuer(20)).await?;
    let locator = state
        .repo
        .validated_projection_locator_for_repo_id(repo_id)?;
    let workspace =
        std::fs::canonicalize(locator.projection_base_abs.join(locator.workspace_segment))?;
    std::fs::write(
        deve_core::utils::notegit::repo_identity_path(&workspace),
        format!(
            "version = 1\nrepo_id = \"{}\"\nrepo_name = \"drifted\"\n",
            RepoId::new_v4()
        ),
    )?;
    let marker_stale = runtime
        .execute_removal(execute_intent(
            &prepared,
            prepared.confirmation_token.clone().expect("token"),
            web_issuer(20),
        ))
        .await
        .expect_err("in-place identity marker drift must invalidate confirmation");
    assert!(matches!(
        marker_stale,
        RepoLifecycleJobError::ConfirmationStale
    ));
    runtime.shutdown().await?;
    drop((state, dir, test_guard));
    Ok(())
}

#[tokio::test]
async fn execute_local_repo_removal_retry_returns_existing_job_or_result() -> anyhow::Result<()> {
    let (test_guard, dir, state) = build_state().await?;
    let repo_id = state.repo.list_cataloged_local_repo_summaries()?[0].repo_id;
    let runtime = state.repo_lifecycle_jobs();
    let prepared = prepare(&runtime, repo_id, Uuid::new_v4(), web_issuer(30)).await?;
    let mut execute = execute_intent(
        &prepared,
        prepared
            .confirmation_token
            .clone()
            .unwrap_or_else(|| panic!("preview blocked: {:?}", prepared.preview)),
        web_issuer(30),
    );
    let first = runtime.execute_removal(execute.clone()).await?;
    let retry = runtime.execute_removal(execute.clone()).await?;
    assert_eq!(retry, first);
    execute.switch_nonce += 1;
    assert!(matches!(
        runtime.execute_removal(execute).await,
        Err(RepoLifecycleJobError::RequestConflict)
    ));
    runtime.shutdown().await?;
    drop((state, dir, test_guard));
    Ok(())
}

#[tokio::test]
async fn removal_request_ids_share_one_namespace_with_prepare_and_create() -> anyhow::Result<()> {
    let (test_guard, dir, state) = build_state().await?;
    let repo_id = state.repo.list_cataloged_local_repo_summaries()?[0].repo_id;
    let runtime = state.repo_lifecycle_jobs();
    let prepared = prepare(&runtime, repo_id, Uuid::new_v4(), web_issuer(35)).await?;

    let mut same_as_prepare = execute_intent(
        &prepared,
        prepared
            .confirmation_token
            .clone()
            .unwrap_or_else(|| panic!("preview blocked: {:?}", prepared.preview)),
        web_issuer(35),
    );
    same_as_prepare.request_id = prepared.request_id;
    assert!(matches!(
        runtime.execute_removal(same_as_prepare).await,
        Err(RepoLifecycleJobError::RequestConflict)
    ));

    let projection = std::fs::canonicalize(dir.path())?;
    let create_request_id = Uuid::new_v4();
    let created = runtime
        .submit(
            create_request_id,
            create_intent(&projection, "Request namespace witness"),
        )
        .await?;
    terminal_status(&runtime, created.request_id).await?;

    assert!(matches!(
        prepare(&runtime, repo_id, create_request_id, web_issuer(36)).await,
        Err(RepoLifecycleJobError::RequestConflict)
    ));
    let mut same_as_create = execute_intent(
        &prepared,
        prepared.confirmation_token.clone().expect("token"),
        web_issuer(35),
    );
    same_as_create.request_id = create_request_id;
    assert!(matches!(
        runtime.execute_removal(same_as_create).await,
        Err(RepoLifecycleJobError::RequestConflict)
    ));
    runtime.shutdown().await?;
    drop((state, dir, test_guard));
    Ok(())
}

#[tokio::test]
async fn lifecycle_store_rejects_cross_record_request_id_collision_on_restart() -> anyhow::Result<()>
{
    let (test_guard, dir, state) = build_state().await?;
    let repo_id = state.repo.list_cataloged_local_repo_summaries()?[0].repo_id;
    let runtime = state.repo_lifecycle_jobs();
    let projection = std::fs::canonicalize(dir.path())?;
    let create_request_id = Uuid::new_v4();
    let created = runtime
        .submit(
            create_request_id,
            create_intent(&projection, "Duplicate namespace witness"),
        )
        .await?;
    terminal_status(&runtime, created.request_id).await?;
    let prepared = prepare(&runtime, repo_id, Uuid::new_v4(), web_issuer(37)).await?;
    runtime.shutdown().await?;

    let record_path = dir
        .path()
        .join("ledger/.host/repo-lifecycle-jobs/removals")
        .join(format!("{}.json", prepared.preparation_id));
    let mut record: serde_json::Value = serde_json::from_slice(&std::fs::read(&record_path)?)?;
    record["prepare_request_id"] = serde_json::json!(create_request_id);
    std::fs::write(&record_path, serde_json::to_vec_pretty(&record)?)?;

    assert!(matches!(
        restart_runtime(&state),
        Err(RepoLifecycleJobError::Store(detail)) if detail.contains("duplicate")
    ));
    drop((state, dir, test_guard));
    Ok(())
}

#[tokio::test]
async fn web_removal_token_binds_principal_connection_and_server_incarnation() -> anyhow::Result<()>
{
    let (test_guard, dir, state) = build_state().await?;
    let repo_id = state.repo.list_cataloged_local_repo_summaries()?[0].repo_id;
    let runtime = state.repo_lifecycle_jobs();
    let prepared = prepare(&runtime, repo_id, Uuid::new_v4(), web_issuer(50)).await?;
    let execute = execute_intent(
        &prepared,
        prepared
            .confirmation_token
            .clone()
            .unwrap_or_else(|| panic!("preview blocked: {:?}", prepared.preview)),
        web_issuer(50),
    );
    let wrong_principal = runtime
        .execute_removal(execute_intent(
            &prepared,
            prepared.confirmation_token.clone().expect("token"),
            web_issuer_with_principal('b', 50),
        ))
        .await
        .expect_err("principal drift must invalidate confirmation");
    assert!(matches!(
        wrong_principal,
        RepoLifecycleJobError::ConfirmationInvalid
    ));
    runtime.shutdown().await?;

    let restarted = restart_runtime(&state)?;
    assert!(matches!(
        restarted.execute_removal(execute).await,
        Err(RepoLifecycleJobError::ConfirmationInvalid)
    ));
    restarted.shutdown().await?;
    drop((state, dir, test_guard));
    Ok(())
}
