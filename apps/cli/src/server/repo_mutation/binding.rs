//! plan_ref:
//!   - 03_storage/authority#repo-mutation-publication-gate
//!
//! Exact local-repository identity validation after acquiring a mutation permit.

use crate::server::{
    AppState,
    repo_scope::{ResolvedRepo, ensure_local_repo_projection_writable},
};
use anyhow::{Context, Result};
use deve_core::models::RepoId;
use std::sync::Arc;

/// Performs the lock-free scope and projection/locator health check that must
/// precede mounted admission. Callers still revalidate the same facts after
/// acquiring the repository mutation permit.
pub(crate) fn prepare_writable_local_repo(
    state: &Arc<AppState>,
    repo_id: RepoId,
    expected_name: &str,
) -> Result<String> {
    resolve_writable_local_repo(
        state,
        repo_id,
        expected_name,
        "before mounted repository admission",
    )
}

/// Rebinds the durable repository identity and rechecks projection writability
/// inside the repo mutation permit. The expected name is diagnostic context,
/// never a fallback that can select a same-name replacement repository.
pub(crate) fn revalidate_writable_local_repo(
    state: &Arc<AppState>,
    repo_id: RepoId,
    expected_name: &str,
) -> Result<String> {
    resolve_writable_local_repo(
        state,
        repo_id,
        expected_name,
        "while waiting for mutation permit",
    )
}

fn resolve_writable_local_repo(
    state: &Arc<AppState>,
    repo_id: RepoId,
    expected_name: &str,
    phase: &str,
) -> Result<String> {
    let repo_name = state
        .repo
        .resolve_local_repo_name_for_execution(Some(repo_id), Some(expected_name))
        .with_context(|| {
            format!("local repository binding changed {phase}: {repo_id} ({expected_name})")
        })?;
    ensure_local_repo_projection_writable(state, &repo_name).map_err(|error| {
        anyhow::anyhow!(
            "local repository projection is not writable: {:?}: {}",
            error.code,
            error.detail.unwrap_or_default()
        )
    })?;
    Ok(repo_name)
}

pub(crate) fn revalidate_writable_resolved_repo(
    state: &Arc<AppState>,
    expected: &ResolvedRepo,
) -> Result<ResolvedRepo> {
    if expected.branch.is_some() {
        anyhow::bail!(
            "local repository mutation cannot target remote branch: {}",
            expected.repo_name
        );
    }
    Ok(ResolvedRepo {
        repo_id: expected.repo_id,
        repo_name: revalidate_writable_local_repo(state, expected.repo_id, &expected.repo_name)?,
        branch: None,
    })
}
