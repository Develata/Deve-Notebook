//! Android producer-bound target and recovery receipt validation.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use super::repo_lifecycle::validate_repo_lifecycle_claims;
use super::{MatrixRow, Receipt};

pub(super) fn validate_android_claims(receipt: &Receipt, row: &MatrixRow) -> Result<()> {
    if receipt.schema != 3 {
        bail!("Android writable evidence requires receipt schema 3 typed claims");
    }
    let claims = receipt
        .claims
        .as_ref()
        .context("Android receipt is missing typed target/probe claims")?;
    if claims.get("schema").and_then(Value::as_u64) != Some(1) {
        bail!("Android claims schema is unsupported");
    }
    let expected_producer = match row.mode.as_str() {
        "local-backend" => "smoke-mobile-android-lifecycle",
        "remote-browser" => "smoke-mobile-android-remote-browser",
        other => bail!("unsupported Android evidence mode {other}"),
    };
    let expected_artifact = match row.mode.as_str() {
        "local-backend" => "smoke-mobile-android-lifecycle.sh",
        "remote-browser" => "smoke-mobile-android-remote-browser.sh",
        _ => unreachable!(),
    };
    if claims.get("producer").and_then(Value::as_str) != Some(expected_producer) {
        bail!("Android claims producer does not match {expected_producer}");
    }
    if !receipt.command_artifacts.iter().any(|artifact| {
        Path::new(artifact)
            .file_name()
            .and_then(|value| value.to_str())
            == Some(expected_artifact)
    }) {
        bail!("Android receipt command is not bound to {expected_artifact}");
    }
    if claims.get("mode").and_then(Value::as_str) != Some(row.mode.as_str()) {
        bail!("Android claims mode does not match receipt mode");
    }
    validate_target_and_webcrypto(claims)?;
    let journey = claims
        .get("journey")
        .context("Android claims journey result is missing")?;
    if journey
        .get("writableLifecycleComplete")
        .and_then(Value::as_bool)
        != Some(true)
    {
        bail!("Android writable lifecycle journey is incomplete");
    }
    validate_repo_lifecycle_claims(claims, journey, "Android")?;
    if row.mode == "remote-browser" {
        validate_remote_journey(journey)?;
        validate_remote_recovery(claims)?;
    } else {
        validate_local_journey(journey)?;
    }
    let executable = Path::new(&receipt.command_program)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(executable.as_str(), "bash" | "bash.exe" | "sh" | "sh.exe") {
        bail!("Android receipt producer must be invoked through the shell harness");
    }
    Ok(())
}

fn validate_local_journey(journey: &Value) -> Result<()> {
    for claim in [
        "loginOrNativeSession",
        "edit",
        "commitHistory",
        "backgroundResume",
        "staleScopeRejected",
        "pendingPreserved",
        "repoRemovalNoScope",
    ] {
        if journey.get(claim).and_then(Value::as_bool) != Some(true) {
            bail!("Android LocalBackend journey is missing required claim {claim}");
        }
    }
    Ok(())
}

fn validate_target_and_webcrypto(claims: &Value) -> Result<()> {
    let target = claims
        .get("target")
        .context("Android claims target is missing")?;
    let sdk = target.get("sdkLevel").and_then(Value::as_u64).unwrap_or(0);
    let provider_major = target
        .get("webViewProviderMajor")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let provider_package = target
        .get("webViewProviderPackage")
        .and_then(Value::as_str)
        .unwrap_or("");
    let provider_version = target
        .get("webViewProviderVersion")
        .and_then(Value::as_str)
        .unwrap_or("");
    let version_major = provider_version
        .split('.')
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let build_fingerprint = target
        .get("buildFingerprint")
        .and_then(Value::as_str)
        .unwrap_or("");
    let model = target.get("model").and_then(Value::as_str).unwrap_or("");
    if sdk < 29
        || provider_major < 137
        || provider_package.is_empty()
        || !provider_package
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_'))
        || provider_version.split('.').count() < 2
        || version_major != provider_major
        || build_fingerprint.is_empty()
        || model.is_empty()
        || target.get("supportBaseline").and_then(Value::as_bool) != Some(true)
    {
        bail!(
            "Android target does not include a qualified API 29 / WebView 137 provider and device identity"
        );
    }
    let webcrypto = claims
        .get("webcrypto")
        .context("Android claims WebCrypto probe is missing")?;
    if webcrypto.get("writable").and_then(Value::as_bool) != Some(true)
        || !webcrypto.get("blocker").is_some_and(Value::is_null)
    {
        bail!("Android target did not pass the real Ed25519 writer probe");
    }
    Ok(())
}

fn validate_remote_journey(journey: &Value) -> Result<()> {
    for claim in [
        "loginOrNativeSession",
        "edit",
        "commitHistory",
        "backgroundResume",
        "repoRemovalNoScope",
        "zeroNativeIpc",
        "nativeLocalRecovery",
        "remoteSurfaceDestroyedBeforeLocalIpc",
        "freshLocalEndpointSessionScope",
        "remoteAuthorityNotReused",
        "noOrphanEmbeddedRuntime",
    ] {
        if journey.get(claim).and_then(Value::as_bool) != Some(true) {
            bail!("Android RemoteBrowser journey is missing required claim {claim}");
        }
    }
    Ok(())
}

fn validate_remote_recovery(claims: &Value) -> Result<()> {
    let recovery = claims
        .get("recovery")
        .context("Android RemoteBrowser claims are missing recovery observations")?;
    let transition = recovery
        .get("transition")
        .context("Android RemoteBrowser recovery transition is missing")?;
    if transition
        .get("recoveryId")
        .and_then(Value::as_u64)
        .is_none_or(|recovery_id| recovery_id == 0)
        || transition.get("phase").and_then(Value::as_str) != Some("local_window_created")
        || transition
            .get("remoteSurfaceRetired")
            .and_then(Value::as_bool)
            != Some(true)
        || transition
            .get("preferenceCommittedAfterRemoteRetirement")
            .and_then(Value::as_bool)
            != Some(true)
        || transition
            .get("localPluginsRegisteredAfterRemoteRetirement")
            .and_then(Value::as_bool)
            != Some(true)
        || transition.get("supervisorManaged").and_then(Value::as_bool) != Some(true)
        || transition
            .get("localWindowCreated")
            .and_then(Value::as_bool)
            != Some(true)
        || transition
            .get("activeRuntimeOwners")
            .and_then(Value::as_u64)
            != Some(1)
        || !transition.get("lastError").is_some_and(Value::is_null)
    {
        bail!("Android RemoteBrowser recovery transition observations are incomplete");
    }
    for observation in [
        "authorityTupleChanged",
        "appPidStable",
        "processExitedAfterGracefulShutdown",
    ] {
        if recovery.get(observation).and_then(Value::as_bool) != Some(true) {
            bail!("Android RemoteBrowser recovery observation {observation} is missing");
        }
    }
    validate_authority_observations(recovery)
}

fn validate_authority_observations(recovery: &Value) -> Result<()> {
    let remote = recovery
        .get("remote")
        .context("Android RemoteBrowser remote authority observation is missing")?;
    let local = recovery
        .get("local")
        .context("Android RemoteBrowser local authority observation is missing")?;
    let remote_origin = remote.get("origin").and_then(Value::as_str).unwrap_or("");
    let local_origin = local.get("origin").and_then(Value::as_str).unwrap_or("");
    let remote_repo = remote.get("repoId").and_then(Value::as_str).unwrap_or("");
    let local_repo = local.get("repoId").and_then(Value::as_str).unwrap_or("");
    let remote_scope = remote
        .get("scopeNonce")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let local_scope = local.get("scopeNonce").and_then(Value::as_u64).unwrap_or(0);
    if !remote_origin.starts_with("https://")
        || local_origin != "http://tauri.localhost"
        || remote_origin == local_origin
        || remote_repo.is_empty()
        || local_repo.is_empty()
        || (remote_repo == local_repo && remote_scope == local_scope)
        || remote_scope == 0
        || local_scope == 0
        || local
            .get("endpoint")
            .and_then(Value::as_str)
            .is_none_or(|endpoint| !endpoint.starts_with("http://127.0.0.1:"))
        || local.get("sessionGeneration").and_then(Value::as_u64) != Some(1)
    {
        bail!("Android RemoteBrowser recovery authority observations are invalid");
    }
    Ok(())
}
