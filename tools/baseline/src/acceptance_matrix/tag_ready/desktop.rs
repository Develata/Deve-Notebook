//! Desktop producer-bound Repo Lifecycle receipt validation.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use anyhow::{Context, Result, bail};
use serde_json::Value;

use super::repo_lifecycle::validate_repo_lifecycle_claims;
use super::{MatrixRow, Receipt};

pub(super) fn validate_desktop_claims(receipt: &Receipt, row: &MatrixRow) -> Result<()> {
    let claims = receipt
        .claims
        .as_ref()
        .context("Desktop Repo Lifecycle receipt is missing typed claims")?;
    if claims.get("schema").and_then(Value::as_u64) != Some(1) {
        bail!("Desktop claims schema is unsupported");
    }
    let expected_producer = match row.mode.as_str() {
        "local-backend" => "smoke-desktop-packaged-ui",
        "remote-browser" => "smoke-desktop-remote-browser",
        other => bail!("unsupported Desktop Repo Lifecycle evidence mode {other}"),
    };
    if claims.get("producer").and_then(Value::as_str) != Some(expected_producer)
        || claims.get("mode").and_then(Value::as_str) != Some(row.mode.as_str())
    {
        bail!("Desktop claims producer or mode does not match the receipt");
    }
    let journey = claims
        .get("journey")
        .context("Desktop claims journey result is missing")?;
    for claim in ["loginOrNativeSession", "edit", "commitHistory"] {
        if journey.get(claim).and_then(Value::as_bool) != Some(true) {
            bail!("Desktop journey is missing required claim {claim}");
        }
    }
    let (removed_repo_id, before) = validate_repo_lifecycle_claims(claims, journey, "Desktop")?;
    let scope = claims
        .get("scope")
        .context("Desktop claims are missing the pre-removal scope")?;
    if scope.get("repoId").and_then(Value::as_str) != Some(removed_repo_id)
        || scope.get("scopeNonce").and_then(Value::as_u64) != Some(before)
    {
        bail!("Desktop pre-removal scope does not match repoLifecycle");
    }
    match row.mode.as_str() {
        "local-backend" => {
            if claims.get("sessionBound").and_then(Value::as_bool) != Some(true) {
                bail!("Desktop LocalBackend claims require a bound native session");
            }
        }
        "remote-browser" => {
            if journey.get("zeroNativeIpc").and_then(Value::as_bool) != Some(true) {
                bail!("Desktop RemoteBrowser claims require zero native IPC");
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}
