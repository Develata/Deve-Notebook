//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! Cross-invocation token and committed-cleanup recovery scenarios.

use super::*;

#[tokio::test]
async fn offline_removal_token_survives_two_cli_invocations_only_for_exact_authority_identity()
-> anyhow::Result<()> {
    let (test_guard, dir, state) = build_state().await?;
    let repo_id = state.repo.list_cataloged_local_repo_summaries()?[0].repo_id;
    let issuer = offline_issuer(&state, repo_id)?;
    assert!(issuer.validate().is_ok(), "offline issuer must start exact");
    let runtime = state.repo_lifecycle_jobs();
    let prepared = prepare(&runtime, repo_id, Uuid::new_v4(), issuer.clone())
        .await
        .context("offline prepare")?;
    let foreign = tempfile::tempdir()?;
    let foreign_lock_path = foreign.path().join("authority.lock");
    std::fs::write(&foreign_lock_path, b"foreign")?;
    let foreign_issuer = RepoRemovalIssuerBinding::OfflineAuthority {
        authority_root: HostPathIdentity::capture(foreign.path(), HostPathKind::Directory)?,
        authority_lock: HostPathIdentity::capture(&foreign_lock_path, HostPathKind::RegularFile)?,
    };
    let foreign_error = runtime
        .execute_removal(execute_intent(
            &prepared,
            prepared.confirmation_token.clone().expect("token"),
            foreign_issuer,
        ))
        .await
        .expect_err("foreign authority root must not consume an offline confirmation");
    assert!(matches!(
        foreign_error,
        RepoLifecycleJobError::ConfirmationInvalid
    ));
    let execute = execute_intent(
        &prepared,
        prepared
            .confirmation_token
            .clone()
            .unwrap_or_else(|| panic!("preview blocked: {:?}", prepared.preview)),
        issuer,
    );
    runtime.shutdown().await?;

    let restarted = restart_runtime(&state)?;
    assert!(
        execute.issuer.validate().is_ok(),
        "offline issuer must remain exact across runtime restart"
    );
    let accepted = restarted
        .execute_removal(execute)
        .await
        .context("offline execute after restart")?;
    let status = terminal_status(&restarted, accepted.request_id).await?;
    assert_eq!(status.outcome, Some(RepoLifecycleJobOutcome::Succeeded));
    restarted.shutdown().await?;
    drop((state, dir, test_guard));
    Ok(())
}

#[tokio::test]
async fn committed_cleanup_debt_is_recovered_from_the_exact_owner_receipt() -> anyhow::Result<()> {
    for (index, step) in super::super::super::RemovalCleanupStep::ORDER
        .into_iter()
        .enumerate()
    {
        let (test_guard, dir, state) = build_state().await?;
        let repo_id = state.repo.list_cataloged_local_repo_summaries()?[0].repo_id;
        let runtime = state.repo_lifecycle_jobs();
        let issuer = web_issuer(60 + index as u64);
        let prepared = prepare(&runtime, repo_id, Uuid::new_v4(), issuer.clone()).await?;
        let execute = execute_intent(
            &prepared,
            prepared
                .confirmation_token
                .clone()
                .unwrap_or_else(|| panic!("missing token for {step:?}: {:?}", prepared.preview)),
            issuer,
        );
        let request_id = execute.request_id;
        state
            .repo_lifecycle_coordinator()
            .fail_next_owned_cleanup_for_test(step);

        runtime.execute_removal(execute).await?;
        timeout(Duration::from_secs(10), async {
            loop {
                let status = runtime.status(request_id).await?;
                let removed = state
                    .repo
                    .repo_catalog_membership_record(repo_id)?
                    .is_some_and(|record| {
                        record.state() == deve_core::ledger::RepoCatalogMembershipState::Removed
                    });
                if removed && status.outcome.is_none() {
                    break Ok::<_, anyhow::Error>(());
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .with_context(|| format!("committed {step:?} debt did not remain active"))??;
        runtime.shutdown().await?;
        drop(runtime);
        drop(state);

        let state = rebuild_cold_host(&dir)?;
        let restarted = state.repo_lifecycle_jobs();
        let recovered = terminal_status(&restarted, request_id).await?;
        assert_eq!(
            recovered.outcome,
            Some(RepoLifecycleJobOutcome::Succeeded),
            "startup recovery must converge {step:?} debt"
        );
        assert!(
            state
                .repo
                .repo_catalog_membership_record(repo_id)?
                .is_none()
        );
        restarted.shutdown().await?;
        drop((state, dir, test_guard));
    }
    Ok(())
}

#[tokio::test]
async fn completed_owner_drift_blocks_later_cleanup_after_cold_restart() -> anyhow::Result<()> {
    let (test_guard, dir, state) = build_state().await?;
    let repo_id = state.repo.list_cataloged_local_repo_summaries()?[0].repo_id;
    let runtime = state.repo_lifecycle_jobs();
    let issuer = web_issuer(88);
    let prepared = prepare(&runtime, repo_id, Uuid::new_v4(), issuer.clone()).await?;
    let execute = execute_intent(
        &prepared,
        prepared
            .confirmation_token
            .clone()
            .unwrap_or_else(|| panic!("missing token: {:?}", prepared.preview)),
        issuer,
    );
    let request_id = execute.request_id;
    state
        .repo_lifecycle_coordinator()
        .fail_next_owned_cleanup_for_test(
            super::super::super::RemovalCleanupStep::ProcessRuntimeSlots,
        );
    runtime.execute_removal(execute).await?;
    timeout(Duration::from_secs(10), async {
        loop {
            if state
                .repo
                .repo_catalog_membership_record(repo_id)?
                .is_some_and(|record| {
                    record.state() == deve_core::ledger::RepoCatalogMembershipState::Removed
                })
                && runtime.status(request_id).await?.outcome.is_none()
            {
                break Ok::<_, anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .context("Remote Import receipt did not precede injected process-runtime failure")??;
    runtime.shutdown().await?;
    drop(runtime);
    drop(state);

    let reappeared_import = dir
        .path()
        .join("ledger/.host/remote-imports")
        .join(repo_id.to_string());
    std::fs::create_dir_all(&reappeared_import)?;
    std::fs::write(reappeared_import.join("foreign"), b"must survive")?;
    let db_path = dir
        .path()
        .join("ledger/local")
        .join(format!("{repo_id}.redb"));

    let state = rebuild_cold_host(&dir)?;
    let restarted = state.repo_lifecycle_jobs();
    tokio::time::sleep(Duration::from_millis(250)).await;
    let status = restarted.status(request_id).await?;
    assert!(
        status.outcome.is_none(),
        "owner drift must remain repair debt"
    );
    assert!(
        state
            .repo
            .repo_catalog_membership_record(repo_id)?
            .is_some_and(|record| {
                record.state() == deve_core::ledger::RepoCatalogMembershipState::Removed
            }),
        "completed-owner drift must not retire the tombstone"
    );
    assert!(db_path.exists(), "later Redb cleanup must not run");
    assert_eq!(
        std::fs::read(reappeared_import.join("foreign"))?,
        b"must survive"
    );
    restarted.shutdown().await?;
    drop((state, dir, test_guard));
    Ok(())
}

#[tokio::test]
async fn authority_retirement_failure_keeps_candidate_unpublished_and_cold_recoverable()
-> anyhow::Result<()> {
    let (test_guard, dir, state) = build_state().await?;
    let repo_id = state.repo.list_cataloged_local_repo_summaries()?[0].repo_id;
    let runtime = state.repo_lifecycle_jobs();
    let mut publications = state.tx.subscribe();
    let issuer = web_issuer(90);
    let prepared = prepare(&runtime, repo_id, Uuid::new_v4(), issuer.clone()).await?;
    let execute = execute_intent(
        &prepared,
        prepared
            .confirmation_token
            .clone()
            .unwrap_or_else(|| panic!("preview blocked: {:?}", prepared.preview)),
        issuer,
    );
    let request_id = execute.request_id;
    state
        .repo_lifecycle_coordinator()
        .fail_next_authority_retirement_for_test();
    runtime.execute_removal(execute).await?;

    let record_path = dir
        .path()
        .join("ledger/.host/repo-lifecycle-jobs/removals")
        .join(format!("{}.json", prepared.preparation_id));
    timeout(Duration::from_secs(10), async {
        loop {
            let text = std::fs::read_to_string(&record_path)?;
            let status = runtime.status(request_id).await?;
            if text.contains("\"state\": \"candidate\"") && status.outcome.is_none() {
                break Ok::<_, anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .context("authority retirement failure did not retain TerminalCandidate debt")??;
    assert!(matches!(
        publications.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));

    runtime.shutdown().await?;
    drop(runtime);
    drop(state);
    let state = rebuild_cold_host(&dir)?;
    let restarted = state.repo_lifecycle_jobs();
    let recovered = terminal_status(&restarted, request_id).await?;
    assert_eq!(recovered.outcome, Some(RepoLifecycleJobOutcome::Succeeded));
    restarted.shutdown().await?;
    drop((state, dir, test_guard));
    Ok(())
}
