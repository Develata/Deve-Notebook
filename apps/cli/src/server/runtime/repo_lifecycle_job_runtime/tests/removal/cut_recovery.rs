//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! Exact crash-cut classification and terminal-finalization recovery.

use super::*;

fn removal_record_path(dir: &tempfile::TempDir, preparation_id: Uuid) -> std::path::PathBuf {
    dir.path()
        .join("ledger/.host/repo-lifecycle-jobs/removals")
        .join(format!("{preparation_id}.json"))
}

async fn wait_for_record(path: &std::path::Path, needle: &str) -> anyhow::Result<String> {
    timeout(Duration::from_secs(10), async {
        loop {
            let text = std::fs::read_to_string(path)?;
            if text.contains(needle) {
                break Ok::<_, anyhow::Error>(text);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .with_context(|| format!("removal record never reached {needle}"))?
}

#[tokio::test]
async fn attempted_cut_with_exact_normal_truth_recovers_not_committed() -> anyhow::Result<()> {
    let (test_guard, dir, state) = build_state().await?;
    let repo_id = state.repo.list_cataloged_local_repo_summaries()?[0].repo_id;
    let runtime = state.repo_lifecycle_jobs();
    let issuer = web_issuer(101);
    let prepared = prepare(&runtime, repo_id, Uuid::new_v4(), issuer.clone()).await?;
    state
        .repo_lifecycle_coordinator()
        .fail_after_cut_attempted_for_test();
    let accepted = runtime
        .execute_removal(execute_intent(
            &prepared,
            prepared.confirmation_token.clone().expect("token"),
            issuer,
        ))
        .await?;
    wait_for_record(
        &removal_record_path(&dir, prepared.preparation_id),
        "\"state\": \"attempted\"",
    )
    .await?;
    assert_eq!(
        state
            .repo
            .repo_catalog_membership_record(repo_id)?
            .expect("normal membership")
            .state(),
        deve_core::ledger::RepoCatalogMembershipState::Normal
    );
    runtime.shutdown().await?;
    drop(runtime);
    drop(state);

    let state = rebuild_cold_host_for_repo(&dir, repo_id)?;
    let restarted = state.repo_lifecycle_jobs();
    let status = terminal_status(&restarted, accepted.request_id).await?;
    assert_eq!(status.outcome, Some(RepoLifecycleJobOutcome::NotCommitted));
    assert_eq!(
        state
            .repo
            .repo_catalog_membership_record(repo_id)?
            .expect("normal membership remains")
            .state(),
        deve_core::ledger::RepoCatalogMembershipState::Normal
    );
    restarted.shutdown().await?;
    drop((state, dir, test_guard));
    Ok(())
}

#[tokio::test]
async fn attempted_cut_with_exact_removed_truth_recovers_committed_cleanup() -> anyhow::Result<()> {
    let (test_guard, dir, state) = build_state().await?;
    let repo_id = state.repo.list_cataloged_local_repo_summaries()?[0].repo_id;
    let runtime = state.repo_lifecycle_jobs();
    let issuer = web_issuer(102);
    let prepared = prepare(&runtime, repo_id, Uuid::new_v4(), issuer.clone()).await?;
    state
        .repo_lifecycle_coordinator()
        .fail_after_catalog_cut_for_test();
    let accepted = runtime
        .execute_removal(execute_intent(
            &prepared,
            prepared.confirmation_token.clone().expect("token"),
            issuer,
        ))
        .await?;
    wait_for_record(
        &removal_record_path(&dir, prepared.preparation_id),
        "\"state\": \"attempted\"",
    )
    .await?;
    assert_eq!(
        state
            .repo
            .repo_catalog_membership_record(repo_id)?
            .expect("removed membership")
            .state(),
        deve_core::ledger::RepoCatalogMembershipState::Removed
    );
    runtime.shutdown().await?;
    drop(runtime);
    drop(state);

    let state = rebuild_cold_host(&dir)?;
    let restarted = state.repo_lifecycle_jobs();
    let status = terminal_status(&restarted, accepted.request_id).await?;
    assert_eq!(status.outcome, Some(RepoLifecycleJobOutcome::Succeeded));
    assert!(
        state
            .repo
            .repo_catalog_membership_record(repo_id)?
            .is_none()
    );
    restarted.shutdown().await?;
    drop((state, dir, test_guard));
    Ok(())
}

#[tokio::test]
async fn attempted_cut_with_changed_normal_truth_remains_repair_debt() -> anyhow::Result<()> {
    let (test_guard, dir, state) = build_state().await?;
    let repo_id = state.repo.list_cataloged_local_repo_summaries()?[0].repo_id;
    let runtime = state.repo_lifecycle_jobs();
    let issuer = web_issuer(103);
    let prepared = prepare(&runtime, repo_id, Uuid::new_v4(), issuer.clone()).await?;
    state
        .repo_lifecycle_coordinator()
        .fail_after_cut_attempted_for_test();
    let accepted = runtime
        .execute_removal(execute_intent(
            &prepared,
            prepared.confirmation_token.clone().expect("token"),
            issuer,
        ))
        .await?;
    wait_for_record(
        &removal_record_path(&dir, prepared.preparation_id),
        "\"state\": \"attempted\"",
    )
    .await?;
    runtime.shutdown().await?;
    drop(runtime);
    drop(state);

    let catalog_path = dir
        .path()
        .join("ledger/.host/repo-catalog")
        .join(format!("{repo_id}.json"));
    let original = std::fs::read_to_string(&catalog_path)?;
    let prefix = "\"membership_revision\":";
    let mut start = original.find(prefix).context("catalog revision field")? + prefix.len();
    while original
        .as_bytes()
        .get(start)
        .is_some_and(u8::is_ascii_whitespace)
    {
        start += 1;
    }
    let end = original[start..]
        .find(|value: char| !value.is_ascii_digit())
        .map(|offset| start + offset)
        .context("catalog revision terminator")?;
    let next = original[start..end].parse::<u64>()? + 1;
    let mut changed = original.clone();
    changed.replace_range(start..end, &next.to_string());
    std::fs::write(&catalog_path, changed)?;

    let state = rebuild_cold_host_for_repo(&dir, repo_id)?;
    let restarted = state.repo_lifecycle_jobs();
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert!(
        restarted
            .status(accepted.request_id)
            .await?
            .outcome
            .is_none()
    );
    assert_eq!(
        state
            .repo
            .repo_catalog_membership_record(repo_id)?
            .expect("changed normal membership")
            .membership_revision(),
        next
    );
    restarted.shutdown().await?;
    drop((state, dir, test_guard));
    Ok(())
}

#[tokio::test]
async fn retired_authority_with_terminal_persist_failure_cold_recovers() -> anyhow::Result<()> {
    let (test_guard, dir, state) = build_state().await?;
    let repo_id = state.repo.list_cataloged_local_repo_summaries()?[0].repo_id;
    let runtime = state.repo_lifecycle_jobs();
    let mut publications = state.tx.subscribe();
    let issuer = web_issuer(104);
    let prepared = prepare(&runtime, repo_id, Uuid::new_v4(), issuer.clone()).await?;
    state
        .repo_lifecycle_coordinator()
        .fail_next_terminal_completion_for_test();
    let accepted = runtime
        .execute_removal(execute_intent(
            &prepared,
            prepared.confirmation_token.clone().expect("token"),
            issuer,
        ))
        .await?;
    let record_path = removal_record_path(&dir, prepared.preparation_id);
    wait_for_record(&record_path, "\"state\": \"candidate\"").await?;
    assert!(matches!(
        publications.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));
    let marker = record_path.parent().expect("removal store").join(
        crate::server::runtime::repo_lifecycle_job_runtime::REMOVAL_PRE_REPLACE_FAILURE_MARKER,
    );
    assert!(marker.is_file());
    let _ = runtime.shutdown().await;
    drop(runtime);
    drop(state);
    std::fs::remove_file(marker)?;

    let state = rebuild_cold_host(&dir)?;
    let restarted = state.repo_lifecycle_jobs();
    let status = terminal_status(&restarted, accepted.request_id).await?;
    assert_eq!(status.outcome, Some(RepoLifecycleJobOutcome::Succeeded));
    restarted.shutdown().await?;
    drop((state, dir, test_guard));
    Ok(())
}
