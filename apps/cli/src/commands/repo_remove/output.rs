//! plan_ref:
//!   - 14_commands#repo-removal-command-contract

use super::token::CliRemovalToken;
use anyhow::{Result, bail};
use deve_core::models::RepoId;
use deve_core::protocol::{LocalRepoRemovalPreview, RepoLifecycleOutcome, RepoLifecycleState};
use serde::Serialize;
use uuid::Uuid;

pub(super) fn prepared(
    repo_id: RepoId,
    preparation_id: Uuid,
    preview: &LocalRepoRemovalPreview,
    token: Option<CliRemovalToken>,
) -> Result<()> {
    println!("repo_removal=preview");
    println!("repo_id={repo_id}");
    println!("deleted={}", labels(&preview.deleted)?);
    println!("preserved={}", labels(&preview.preserved)?);
    println!("warnings={}", labels(&preview.warnings)?);
    println!("blockers={}", labels(&preview.blockers)?);
    match token {
        Some(token) => println!("confirmation_token={}", token.encode()),
        None => println!("confirmation_token=unavailable"),
    }
    println!("preparation_id={preparation_id}");
    Ok(())
}

pub(super) fn accepted(repo_id: RepoId, request_id: Uuid, job_id: Uuid) {
    println!("repo_removal=accepted");
    println!("repo_id={repo_id}");
    println!("request_id={request_id}");
    println!("job_id={job_id}");
}

pub(super) fn terminal(
    repo_id: RepoId,
    request_id: Uuid,
    state: RepoLifecycleState,
    outcome: Option<RepoLifecycleOutcome>,
    publication_pending: bool,
) -> Result<()> {
    println!("repo_removal=terminal");
    println!("repo_id={repo_id}");
    println!("request_id={request_id}");
    println!("state={}", label(&state)?);
    println!(
        "outcome={}",
        outcome
            .as_ref()
            .map(label)
            .transpose()?
            .unwrap_or_else(|| "unknown".to_string())
    );
    println!("publication_pending={publication_pending}");
    if state != RepoLifecycleState::Terminal
        || outcome != Some(RepoLifecycleOutcome::Succeeded)
        || publication_pending
    {
        bail!("REPO_LIFECYCLE_REPAIR_REQUIRED");
    }
    Ok(())
}

fn labels<T: Serialize>(values: &[T]) -> Result<String> {
    values
        .iter()
        .map(label)
        .collect::<Result<Vec<_>>>()
        .map(|v| v.join(","))
}

fn label<T: Serialize>(value: &T) -> Result<String> {
    let json = serde_json::to_string(value)?;
    Ok(json.trim_matches('"').to_string())
}
