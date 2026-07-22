//! plan_ref:
//!   - 04_repository#local-repo-removal-contract
//!
//! End-to-end owner cleanup and workspace-preservation proof.

use super::*;

#[tokio::test]
async fn execute_local_repo_removal_atomically_persists_admission_before_worker()
-> anyhow::Result<()> {
    let (test_guard, dir, state) = build_state().await?;
    let repo_id = state.repo.list_cataloged_local_repo_summaries()?[0].repo_id;
    let locator = state
        .repo
        .validated_projection_locator_for_repo_id(repo_id)?;
    let workspace =
        std::fs::canonicalize(locator.projection_base_abs.join(&locator.workspace_segment))?;
    let markdown = workspace.join("preserved.md");
    let git_head = workspace.join(".git/HEAD");
    std::fs::write(&markdown, "# preserved\n")?;
    std::fs::create_dir_all(git_head.parent().expect("git parent"))?;
    std::fs::write(&git_head, "ref: refs/heads/main\n")?;
    let authority = state.repo.snapshot_local_authority_for_removal(repo_id)?;
    let locator_plan = state.repo.prepare_projection_locator_removal(repo_id)?;
    let alias_plan = state
        .repo
        .host_repo_alias_runtime()
        .prepare_removal(repo_id)?;
    let database = authority.database().path().to_path_buf();
    let authority_lock = authority.authority_lock().path().to_path_buf();
    let runtime = state.repo_lifecycle_jobs();
    timeout(Duration::from_secs(10), async {
        loop {
            if state.watcher_runtime_view().admit(repo_id).is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .context("watcher did not reach Mounted before removal preview")?;
    let prepared = prepare(&runtime, repo_id, Uuid::new_v4(), web_issuer(40)).await?;
    let execute = execute_intent(
        &prepared,
        prepared
            .confirmation_token
            .clone()
            .unwrap_or_else(|| panic!("preservation preview blocked: {:?}", prepared.preview)),
        web_issuer(40),
    );
    let accepted = runtime.execute_removal(execute).await?;
    let request_id = accepted.request_id;
    let record_path = dir
        .path()
        .join("ledger/.host/repo-lifecycle-jobs/removals")
        .join(format!("{}.json", prepared.preparation_id));
    assert!(std::fs::read_to_string(&record_path)?.contains("execute_admitted"));

    let status = terminal_status(&runtime, request_id).await?;
    assert_eq!(
        status.outcome,
        Some(RepoLifecycleJobOutcome::Succeeded),
        "unexpected removal status: {status:?}"
    );
    assert!(
        state
            .repo
            .repo_catalog_membership_record(repo_id)?
            .is_none()
    );
    assert!(
        !database.exists(),
        "canonical Redb authority must be deleted"
    );
    assert!(
        authority_lock.is_file(),
        "persistent authority lock pathname is host identity and must remain"
    );
    assert!(workspace.is_dir(), "workspace root must be preserved");
    assert!(markdown.is_file(), "Markdown must be preserved");
    assert!(git_head.is_file(), ".git must be preserved");
    assert!(!workspace.join(".notegit").exists());
    assert!(
        state
            .repo
            .projection_locator_removal_is_absent(&locator_plan)?,
        "exact projection locator row must be absent"
    );
    assert!(
        state
            .repo
            .host_repo_alias_runtime()
            .removal_is_absent(&alias_plan)?,
        "exact host alias row must be absent"
    );
    assert!(
        !dir.path()
            .join("ledger/.host/remote-imports")
            .join(repo_id.to_string())
            .exists(),
        "Remote Import owner root must be absent"
    );
    assert!(
        !dir.path()
            .join("ledger/.host/repo-catalog")
            .join(format!("{repo_id}.json"))
            .exists(),
        "catalog tombstone must be retired"
    );
    runtime.shutdown().await?;
    drop((state, dir, test_guard));
    Ok(())
}
