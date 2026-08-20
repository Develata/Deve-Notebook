//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::{
    Receipt, ReceiptProducerBinding, ReceiptRecord, consume_accepted_gap,
    validate_execution_groups, validate_receipt,
};
use crate::acceptance_matrix::model::MatrixRow;
use chrono::{Duration, Utc};
use serde_json::json;
use std::collections::BTreeMap;

fn row() -> MatrixRow {
    MatrixRow {
        requirement_id: "journey.android".into(),
        journey_id: "android-local-backend".into(),
        flow_id: "none".into(),
        case_id: "none".into(),
        surface: "android".into(),
        mode: "local-backend".into(),
        gate: "tag-ready".into(),
        requirement: "required".into(),
        evidence_kind: "receipt".into(),
        evidence_id: "smoke.android".into(),
        evidence_ref: "receipts/smoke.android.json".into(),
        freshness: "target-host-30d".into(),
        note: String::new(),
    }
}

#[test]
fn accepted_gap_consumption_requires_the_exact_requirement_and_evidence_pair() {
    let mut gap_row = row();
    gap_row.requirement_id = "case.store-016".into();
    gap_row.evidence_kind = "gap".into();
    gap_row.evidence_id = "gap.watcher.windows-overflow-receipt".into();

    let key = (gap_row.requirement_id.clone(), gap_row.evidence_id.clone());
    let mut bindings = BTreeMap::from([(
        key.clone(),
        "known-limitation.windows-watcher-overflow".into(),
    )]);
    assert_eq!(
        consume_accepted_gap(&gap_row, &mut bindings).as_deref(),
        Some("known-limitation.windows-watcher-overflow")
    );
    assert!(bindings.is_empty());

    let mut wrong_row = gap_row;
    wrong_row.evidence_id = "gap.watcher.other".into();
    let mut bindings = BTreeMap::from([(key, "known-limitation.windows-watcher-overflow".into())]);
    assert_eq!(consume_accepted_gap(&wrong_row, &mut bindings), None);
    assert_eq!(bindings.len(), 1);
}

fn binding() -> ReceiptProducerBinding {
    ReceiptProducerBinding {
        producer_id: "android.local-backend".into(),
        contract_fingerprint: "fnv1a64:1111111111111111".into(),
        evidence_ids: vec!["smoke.android".into()],
        artifacts: vec!["scripts/smoke-mobile-android-lifecycle.sh".into()],
        bound_env: Vec::new(),
    }
}

fn record(now: chrono::DateTime<Utc>) -> ReceiptRecord {
    ReceiptRecord {
        relative_path: "receipts/smoke.android.json".into(),
        receipt: Receipt {
            schema: 3,
            producer_id: "android.local-backend".into(),
            producer_contract: "fnv1a64:1111111111111111".into(),
            execution_id: "exec-fnv1a64-2222222222222222".into(),
            execution_evidence_ids: vec!["smoke.android".into()],
            evidence_id: "smoke.android".into(),
            evidence_ref: "receipts/smoke.android.json".into(),
            head: "abc".into(),
            head_after: Some("abc".into()),
            dirty_before: false,
            dirty_after: false,
            os: "linux".into(),
            arch: "x86_64".into(),
            target_os: "android".into(),
            surface: "android".into(),
            mode: "local-backend".into(),
            started_at: (now - Duration::minutes(1)).to_rfc3339(),
            finished_at: now.to_rfc3339(),
            status: "passed".into(),
            exit_code: Some(0),
            error: None,
            command_program: "bash".into(),
            command_arg_count: 1,
            command_fingerprint: "fnv1a64:0123456789abcdef".into(),
            command_artifacts: vec!["scripts/smoke-mobile-android-lifecycle.sh".into()],
            producer_inputs: BTreeMap::new(),
            claims: Some(json!({
                "schema": 1,
                "producer": "smoke-mobile-android-lifecycle",
                "mode": "local-backend",
                "target": {
                    "sdkLevel": 29,
                    "webViewProviderPackage": "com.google.android.webview",
                    "webViewProviderVersion": "137.0.7151.115",
                    "webViewProviderMajor": 137,
                    "avdName": "deve-api37",
                    "buildFingerprint": "google/sdk_gphone64_x86_64/test",
                    "model": "sdk_gphone64_x86_64",
                    "supportBaseline": true
                },
                "webcrypto": { "writable": true, "blocker": null },
                "repoLifecycle": {
                    "removedRepoId": "repo-android",
                    "scopeNonceBeforeRemoval": 7,
                    "scopeNonceAfterRemoval": 8,
                    "noScope": true
                },
                "journey": {
                    "loginOrNativeSession": true,
                    "edit": true,
                    "commitHistory": true,
                    "backgroundResume": true,
                    "staleScopeRejected": true,
                    "pendingPreserved": true,
                    "repoRemovalNoScope": true,
                    "rootBackBackgroundsTaskWithStablePid": true,
                    "writableLifecycleComplete": true
                }
            })),
        },
    }
}

fn desktop_fixture(
    now: chrono::DateTime<Utc>,
    mode: &str,
) -> (MatrixRow, ReceiptProducerBinding, ReceiptRecord) {
    let mut row = row();
    row.requirement_id = format!("journey.desktop.{mode}");
    row.journey_id = format!("desktop-{mode}");
    row.surface = "desktop".into();
    row.mode = mode.into();
    row.evidence_id = format!("smoke.desktop.{mode}");
    row.evidence_ref = format!("receipts/smoke.desktop.{mode}.json");

    let producer_id = format!("desktop.{mode}");
    let mut binding = binding();
    binding.producer_id = producer_id.clone();
    binding.evidence_ids = vec![row.evidence_id.clone()];
    binding.artifacts = vec![format!("scripts/check-desktop-{mode}-smoke.ps1")];

    let claims_producer = match mode {
        "local-backend" => "smoke-desktop-packaged-ui",
        "remote-browser" => "smoke-desktop-remote-browser",
        _ => panic!("unsupported test mode"),
    };
    let mut record = record(now);
    record.relative_path = row.evidence_ref.clone();
    record.receipt.producer_id = producer_id;
    record.receipt.execution_evidence_ids = vec![row.evidence_id.clone()];
    record.receipt.evidence_id = row.evidence_id.clone();
    record.receipt.evidence_ref = row.evidence_ref.clone();
    record.receipt.os = "windows".into();
    record.receipt.target_os = "windows".into();
    record.receipt.surface = "desktop".into();
    record.receipt.mode = mode.into();
    record.receipt.command_program = "pwsh".into();
    record.receipt.command_artifacts = binding.artifacts.clone();
    record.receipt.claims = Some(json!({
        "schema": 1,
        "producer": claims_producer,
        "mode": mode,
        "origin": if mode == "remote-browser" {
            "https://desktop.example.test"
        } else {
            "tauri://localhost"
        },
        "httpBase": "http://127.0.0.1:39123",
        "sessionBound": true,
        "scope": {
            "repoId": "repo-desktop",
            "scopeNonce": 11
        },
        "repoLifecycle": {
            "removedRepoId": "repo-desktop",
            "scopeNonceBeforeRemoval": 11,
            "scopeNonceAfterRemoval": 12,
            "noScope": true
        },
        "journey": {
            "loginOrNativeSession": true,
            "edit": true,
            "commitHistory": true,
            "zeroNativeIpc": true,
            "repoRemovalNoScope": true
        }
    }));
    (row, binding, record)
}

fn android_remote_fixture(
    now: chrono::DateTime<Utc>,
) -> (MatrixRow, ReceiptProducerBinding, ReceiptRecord) {
    let mut row = row();
    row.requirement_id = "journey.android.remote-browser".into();
    row.journey_id = "android-remote-browser".into();
    row.mode = "remote-browser".into();

    let mut binding = binding();
    binding.producer_id = "android.remote-browser".into();
    binding.artifacts = vec!["scripts/smoke-mobile-android-remote-browser.sh".into()];

    let mut record = record(now);
    record.receipt.producer_id = binding.producer_id.clone();
    record.receipt.mode = row.mode.clone();
    record.receipt.command_artifacts = binding.artifacts.clone();
    let claims = record.receipt.claims.as_mut().expect("claims");
    claims["producer"] = json!("smoke-mobile-android-remote-browser");
    claims["mode"] = json!("remote-browser");
    claims["journey"] = json!({
        "loginOrNativeSession": true,
        "edit": true,
        "commitHistory": true,
        "backgroundResume": true,
        "repoRemovalNoScope": true,
        "zeroNativeIpc": true,
        "nativeLocalRecovery": true,
        "remoteSurfaceDestroyedBeforeLocalIpc": true,
        "freshLocalBootstrapUnboundBeforeFirstCreate": true,
        "freshLocalEndpointSessionScope": true,
        "remoteAuthorityNotReused": true,
        "noOrphanEmbeddedRuntime": true,
        "writableLifecycleComplete": true
    });
    claims["recovery"] = json!({
        "transition": {
            "recoveryId": 1,
            "phase": "local_window_created",
            "remoteSurfaceRetired": true,
            "preferenceCommittedAfterRemoteRetirement": true,
            "localPluginsRegisteredAfterRemoteRetirement": true,
            "supervisorManaged": true,
            "localWindowCreated": true,
            "activeRuntimeOwners": 1,
            "lastError": null
        },
        "authorityTupleChanged": true,
        "appPidStable": true,
        "processExitedAfterGracefulShutdown": true,
        "remote": {
            "origin": "https://android.example.test",
            "repoId": "repo-android",
            "scopeNonce": 7
        },
        "local": {
            "origin": "http://tauri.localhost",
            "endpoint": "http://127.0.0.1:39123",
            "bootstrapUnbound": {
                "status": "handshaking-repo",
                "repoIdEmpty": true,
                "scopeNonce": 0,
                "defaultRepoAbsent": true
            },
            "status": "ready",
            "repoId": "repo-local",
            "scopeNonce": 9,
            "sessionGeneration": 1
        }
    });
    (row, binding, record)
}

fn validate_fixture(
    record: &ReceiptRecord,
    row: &MatrixRow,
    binding: &ReceiptProducerBinding,
) -> Result<(), anyhow::Error> {
    validate_execution_groups([&record.receipt], "acceptance-matrix tag-ready fixture")?;
    validate_receipt(record, row, binding, "abc", Utc::now())
}

#[test]
fn target_host_receipt_requires_current_producer_contract() {
    let now = Utc::now();
    let row = row();
    let binding = binding();
    let mut record = record(now);
    assert!(validate_fixture(&record, &row, &binding).is_ok());
    record.receipt.producer_id = "manual.unbound".into();
    assert!(validate_fixture(&record, &row, &binding).is_err());
    record.receipt.producer_id = binding.producer_id.clone();
    record.receipt.producer_contract = "fnv1a64:9999999999999999".into();
    assert!(validate_fixture(&record, &row, &binding).is_err());
}

#[test]
fn target_host_receipt_rejects_dirty_stale_and_future_ranges() {
    let now = Utc::now();
    let row = row();
    let binding = binding();
    let mut record = record(now);
    record.receipt.dirty_before = true;
    assert!(validate_fixture(&record, &row, &binding).is_err());
    record.receipt.dirty_before = false;
    record.receipt.started_at = (now - Duration::days(31)).to_rfc3339();
    record.receipt.finished_at = (now - Duration::days(31)).to_rfc3339();
    assert!(validate_fixture(&record, &row, &binding).is_err());
    record.receipt.started_at = (now + Duration::minutes(2)).to_rfc3339();
    record.receipt.finished_at = now.to_rfc3339();
    assert!(validate_fixture(&record, &row, &binding).is_err());
}

#[test]
fn android_receipt_requires_repo_removal_no_scope_claim() {
    let now = Utc::now();
    let row = row();
    let binding = binding();
    let mut record = record(now);
    record
        .receipt
        .claims
        .as_mut()
        .and_then(|claims| claims.get_mut("journey"))
        .and_then(|journey| journey.as_object_mut())
        .expect("journey claims")
        .remove("repoRemovalNoScope");
    assert!(validate_fixture(&record, &row, &binding).is_err());
}

#[test]
fn android_receipt_requires_root_back_same_pid_claim() {
    let now = Utc::now();
    let row = row();
    let binding = binding();
    let mut record = record(now);
    assert!(validate_fixture(&record, &row, &binding).is_ok());
    record
        .receipt
        .claims
        .as_mut()
        .and_then(|claims| claims.get_mut("journey"))
        .and_then(|journey| journey.as_object_mut())
        .expect("journey claims")
        .remove("rootBackBackgroundsTaskWithStablePid");
    assert!(validate_fixture(&record, &row, &binding).is_err());
    record.receipt.claims.as_mut().expect("claims")["journey"]["rootBackBackgroundsTaskWithStablePid"] =
        json!(true);
    assert!(validate_fixture(&record, &row, &binding).is_ok());
    record.receipt.claims.as_mut().expect("claims")["journey"]["rootBackBackgroundsTaskWithStablePid"] =
        json!(false);
    assert!(validate_fixture(&record, &row, &binding).is_err());
}

#[test]
fn android_receipt_requires_monotonic_repo_lifecycle_claims() {
    let now = Utc::now();
    let row = row();
    let binding = binding();
    let mut record = record(now);
    record.receipt.claims.as_mut().expect("claims")["repoLifecycle"]["scopeNonceAfterRemoval"] =
        json!(7);
    assert!(validate_fixture(&record, &row, &binding).is_err());
}

#[test]
fn android_remote_receipt_requires_repo_removal_no_scope_claim() {
    let now = Utc::now();
    let (row, binding, mut record) = android_remote_fixture(now);
    assert!(validate_fixture(&record, &row, &binding).is_ok());
    record
        .receipt
        .claims
        .as_mut()
        .and_then(|claims| claims.get_mut("journey"))
        .and_then(|journey| journey.as_object_mut())
        .expect("remote journey claims")
        .remove("repoRemovalNoScope");
    assert!(validate_fixture(&record, &row, &binding).is_err());
}

#[test]
fn android_remote_receipt_requires_observed_retirement_and_bundled_local_target() {
    let now = Utc::now();
    let (row, binding, mut record) = android_remote_fixture(now);
    assert!(validate_fixture(&record, &row, &binding).is_ok());

    record.receipt.claims.as_mut().expect("claims")["recovery"]["transition"]
        .as_object_mut()
        .expect("transition")
        .remove("recoveryId");
    assert!(validate_fixture(&record, &row, &binding).is_err());

    let (_, _, mut record) = android_remote_fixture(now);
    record.receipt.claims.as_mut().expect("claims")["recovery"]["transition"]["recoveryId"] =
        json!(0);
    assert!(validate_fixture(&record, &row, &binding).is_err());

    let (_, _, mut record) = android_remote_fixture(now);
    record.receipt.claims.as_mut().expect("claims")["recovery"]["transition"]["remoteSurfaceRetired"] =
        json!(false);
    record.receipt.claims.as_mut().expect("claims")["recovery"]["remoteTargetRetired"] =
        json!(true);
    assert!(validate_fixture(&record, &row, &binding).is_err());

    let (_, _, mut record) = android_remote_fixture(now);
    record.receipt.claims.as_mut().expect("claims")["recovery"]["local"]["origin"] =
        json!("https://stale-remote.example.test");
    assert!(validate_fixture(&record, &row, &binding).is_err());
}

#[test]
fn android_remote_receipt_requires_bootstrap_unbound_then_ready_first_create() {
    let now = Utc::now();
    let (row, binding, record) = android_remote_fixture(now);
    assert!(validate_fixture(&record, &row, &binding).is_ok());

    let (_, _, mut record) = android_remote_fixture(now);
    record.receipt.claims.as_mut().expect("claims")["journey"]
        .as_object_mut()
        .expect("journey")
        .remove("freshLocalBootstrapUnboundBeforeFirstCreate");
    assert!(validate_fixture(&record, &row, &binding).is_err());

    let (_, _, mut record) = android_remote_fixture(now);
    record.receipt.claims.as_mut().expect("claims")["journey"]["freshLocalBootstrapUnboundBeforeFirstCreate"] =
        json!(false);
    assert!(validate_fixture(&record, &row, &binding).is_err());

    let (_, _, mut record) = android_remote_fixture(now);
    record.receipt.claims.as_mut().expect("claims")["recovery"]["local"]
        .as_object_mut()
        .expect("local recovery")
        .remove("bootstrapUnbound");
    assert!(validate_fixture(&record, &row, &binding).is_err());

    for (field, invalid) in [
        ("status", json!("ready")),
        ("repoIdEmpty", json!(false)),
        ("scopeNonce", json!(1)),
        ("defaultRepoAbsent", json!(false)),
    ] {
        let (_, _, mut record) = android_remote_fixture(now);
        record.receipt.claims.as_mut().expect("claims")["recovery"]["local"]["bootstrapUnbound"]
            [field] = invalid;
        assert!(validate_fixture(&record, &row, &binding).is_err());
    }

    let (_, _, mut record) = android_remote_fixture(now);
    record.receipt.claims.as_mut().expect("claims")["recovery"]["local"]["status"] =
        json!("handshaking-repo");
    assert!(validate_fixture(&record, &row, &binding).is_err());
}

#[test]
fn desktop_repo_lifecycle_receipts_require_typed_claims() {
    let now = Utc::now();
    for mode in ["local-backend", "remote-browser"] {
        let (row, binding, record) = desktop_fixture(now, mode);
        assert!(validate_fixture(&record, &row, &binding).is_ok());
    }
}

#[test]
fn desktop_repo_lifecycle_receipt_rejects_false_or_inconsistent_claims() {
    let now = Utc::now();
    let (row, binding, mut record) = desktop_fixture(now, "remote-browser");
    record.receipt.claims.as_mut().expect("claims")["journey"]["zeroNativeIpc"] = json!(false);
    assert!(validate_fixture(&record, &row, &binding).is_err());

    let (_, _, mut record) = desktop_fixture(now, "remote-browser");
    record.receipt.claims.as_mut().expect("claims")["repoLifecycle"]["scopeNonceAfterRemoval"] =
        json!(11);
    assert!(validate_fixture(&record, &row, &binding).is_err());

    let (_, _, mut record) = desktop_fixture(now, "remote-browser");
    record.receipt.claims.as_mut().expect("claims")["scope"]["repoId"] = json!("wrong-repo");
    assert!(validate_fixture(&record, &row, &binding).is_err());
}

#[test]
fn execution_group_requires_every_declared_sibling() {
    let now = Utc::now();
    let row = row();
    let mut binding = binding();
    binding.evidence_ids.push("smoke.android.second".into());
    let mut record = record(now);
    record.receipt.execution_evidence_ids = binding.evidence_ids.clone();
    assert!(validate_fixture(&record, &row, &binding).is_err());
}
