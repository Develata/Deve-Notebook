//! Shared typed Repo Lifecycle claims validation.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use anyhow::{Context, Result, bail};
use serde_json::Value;

pub(super) fn validate_repo_lifecycle_claims<'a>(
    claims: &'a Value,
    journey: &Value,
    label: &str,
) -> Result<(&'a str, u64)> {
    let lifecycle = claims
        .get("repoLifecycle")
        .with_context(|| format!("{label} claims are missing repoLifecycle"))?;
    let removed_repo_id = lifecycle
        .get("removedRepoId")
        .and_then(Value::as_str)
        .unwrap_or("");
    let before = lifecycle
        .get("scopeNonceBeforeRemoval")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let after = lifecycle
        .get("scopeNonceAfterRemoval")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if removed_repo_id.is_empty()
        || before == 0
        || after <= before
        || lifecycle.get("noScope").and_then(Value::as_bool) != Some(true)
        || journey.get("repoRemovalNoScope").and_then(Value::as_bool) != Some(true)
    {
        bail!("{label} claims do not prove typed last-repo NoScope finalization");
    }
    Ok((removed_repo_id, before))
}
