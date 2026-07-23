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
        .execute_removal(execute.clone())
        .await
        .context("offline execute after restart")?;
    let status = terminal_status(&restarted, accepted.request_id).await?;
    assert_eq!(status.outcome, Some(RepoLifecycleJobOutcome::Succeeded));
    restarted.shutdown().await?;
    let replay_runtime = restart_runtime(&state)?;
    let replayed = replay_runtime
        .execute_removal(execute)
        .await
        .context("offline execute lost-response replay after terminal restart")?;
    assert_eq!(replayed, accepted);
    let replayed_status = terminal_status(&replay_runtime, replayed.request_id).await?;
    assert_eq!(
        replayed_status.outcome,
        Some(RepoLifecycleJobOutcome::Succeeded)
    );
    replay_runtime.shutdown().await?;
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
                if removed && status.outcome == Some(RepoLifecycleJobOutcome::RepairRequired) {
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
async fn explicit_repair_reissues_expires_and_consumes_one_shot_authorization() -> anyhow::Result<()>
{
    let (test_guard, dir, state) = build_state().await?;
    let repo_id = state.repo.list_cataloged_local_repo_summaries()?[0].repo_id;
    let runtime = state.repo_lifecycle_jobs();
    let issuer = web_issuer(77);
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
    let debt = terminal_status(&runtime, request_id).await?;
    assert_eq!(debt.outcome, Some(RepoLifecycleJobOutcome::RepairRequired));

    let repair_issuer = local_proxy_repair_issuer(&runtime, 'b');
    let first = runtime
        .prepare_removal_repair(request_id, repair_issuer.clone())
        .await?;
    assert!(first.inspection.apply_allowed);
    assert!(!first.inspection.remaining.is_empty());
    let first_token = first.token.expect("first repair token");
    let second = runtime
        .prepare_removal_repair(request_id, repair_issuer.clone())
        .await?;
    let second_token = second.token.expect("second repair token");
    let before_expiry = second.expires_at_unix_ms.expect("repair expiry") - 1;
    let superseded = runtime
        .apply_removal_repair_at_for_test(
            RepoRemovalRepairApplyIntent {
                request_id,
                token: first_token,
                issuer: repair_issuer.clone(),
            },
            before_expiry,
        )
        .await
        .expect_err("reissued preview must invalidate old token");
    assert!(matches!(
        superseded,
        RepoLifecycleJobError::ConfirmationStale
    ));
    let expired = runtime
        .apply_removal_repair_at_for_test(
            RepoRemovalRepairApplyIntent {
                request_id,
                token: second_token,
                issuer: repair_issuer.clone(),
            },
            second.expires_at_unix_ms.expect("repair expiry") + 1,
        )
        .await
        .expect_err("expired repair token must fail");
    assert!(matches!(
        expired,
        RepoLifecycleJobError::ConfirmationExpired
    ));

    let current = runtime
        .prepare_removal_repair(request_id, repair_issuer.clone())
        .await?;
    let current_token = current.token.expect("current repair token");
    let foreign = runtime
        .apply_removal_repair(RepoRemovalRepairApplyIntent {
            request_id,
            token: current_token.clone(),
            issuer: local_proxy_repair_issuer(&runtime, 'd'),
        })
        .await
        .expect_err("repair token must bind its exact operator and runtime");
    assert!(matches!(foreign, RepoLifecycleJobError::ConfirmationStale));
    let accepted = runtime
        .apply_removal_repair(RepoRemovalRepairApplyIntent {
            request_id,
            token: current_token.clone(),
            issuer: repair_issuer.clone(),
        })
        .await?;
    assert_eq!(accepted.target_repo_id, repo_id);
    let replayed_while_running = runtime
        .apply_removal_repair(RepoRemovalRepairApplyIntent {
            request_id,
            token: current_token.clone(),
            issuer: repair_issuer.clone(),
        })
        .await?;
    assert_eq!(replayed_while_running.request_id, accepted.request_id);
    assert_eq!(replayed_while_running.job_id, accepted.job_id);
    let recovered = terminal_status(&runtime, request_id).await?;
    assert_eq!(recovered.outcome, Some(RepoLifecycleJobOutcome::Succeeded));
    let replayed_after_terminal = runtime
        .apply_removal_repair(RepoRemovalRepairApplyIntent {
            request_id,
            token: current_token,
            issuer: repair_issuer,
        })
        .await?;
    assert_eq!(replayed_after_terminal.request_id, accepted.request_id);
    assert_eq!(replayed_after_terminal.job_id, accepted.job_id);
    runtime.shutdown().await?;
    drop((state, dir, test_guard));
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
                && matches!(
                    runtime.status(request_id).await?.outcome,
                    None | Some(RepoLifecycleJobOutcome::RepairRequired)
                )
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
    assert_eq!(
        status.outcome,
        Some(RepoLifecycleJobOutcome::RepairRequired),
        "owner drift must remain visible as repair debt"
    );
    let repair = restarted
        .prepare_removal_repair(request_id, local_proxy_repair_issuer(&restarted, 'c'))
        .await?;
    assert!(!repair.inspection.apply_allowed);
    assert!(repair.token.is_none());
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
            if text.contains("\"state\": \"candidate\"")
                && status.outcome == Some(RepoLifecycleJobOutcome::RepairRequired)
            {
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
