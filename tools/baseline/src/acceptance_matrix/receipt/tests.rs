use super::executor::{command_fingerprint, read_claims, serialize_receipts_bounded};
use super::{CommandStep, EvidenceSpec, ExecutionSpec, ReceiptArgs, execute_and_write, run};
use crate::acceptance_matrix::receipt_limits::{
    MAX_EXECUTION_RECEIPTS, MAX_RECEIPT_BYTES, MAX_TOTAL_RECEIPT_BYTES,
};
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn unique_output(label: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("deve-{label}-{unique}.json"))
}

#[test]
fn receipt_args_require_explicit_surface_and_command_boundary() {
    let output = unique_output("receipt-args");
    let args = vec![
        "--evidence-id".into(),
        "smoke.web".into(),
        "--evidence-ref".into(),
        output.file_name().unwrap().to_string_lossy().into_owned(),
        "--surface".into(),
        "web".into(),
        "--mode".into(),
        "browser".into(),
        "--target-os".into(),
        "web".into(),
        "--output".into(),
        output.display().to_string(),
        "--timeout-secs".into(),
        "2".into(),
        "--".into(),
        "cargo".into(),
        "test".into(),
    ];
    let parsed = ReceiptArgs::parse(&args).unwrap();
    assert_eq!(parsed.evidence.surface, "web");
    assert_eq!(parsed.evidence.mode, "browser");
    assert_eq!(parsed.command, ["cargo", "test"]);
    assert_eq!(parsed.timeout_secs, 2);
    assert_eq!(
        command_fingerprint(&parsed.command),
        command_fingerprint(&parsed.command)
    );
}

#[test]
fn failed_to_start_command_still_writes_failed_receipt() {
    let output = unique_output("acceptance-receipt");
    let file_name = output.file_name().unwrap().to_string_lossy().to_string();
    let args = receipt_args(&output, &file_name, "test.failed-command");
    let mut args = args;
    args.push("deve-command-that-must-not-exist".into());
    assert!(run(&args).is_err());
    let receipt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(receipt["status"], "failed");
    assert!(receipt["exit_code"].is_null());
    let _ = fs::remove_file(output);
}

#[test]
fn timed_out_command_writes_failed_receipt() {
    let output = unique_output("acceptance-timeout");
    let file_name = output.file_name().unwrap().to_string_lossy().to_string();
    let (program, command_args): (&str, Vec<&str>) = if cfg!(windows) {
        (
            "powershell",
            vec!["-NoProfile", "-Command", "Start-Sleep -Seconds 5"],
        )
    } else {
        ("sh", vec!["-c", "sleep 5"])
    };
    let mut args = receipt_args(&output, &file_name, "test.timeout");
    args.splice(
        args.len() - 1..args.len() - 1,
        ["--timeout-secs".to_string(), "1".to_string()],
    );
    args.push(program.to_string());
    args.extend(command_args.into_iter().map(str::to_string));
    assert!(run(&args).is_err());
    let receipt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(receipt["status"], "failed");
    assert!(receipt["error"].as_str().unwrap().contains("exceeded"));
    let _ = fs::remove_file(output);
}

#[test]
fn oversized_claims_are_rejected_without_loading_or_advancing_the_group_budget() {
    let claims = unique_output("oversized-claims");
    let file = fs::File::create(&claims).unwrap();
    file.set_len(MAX_RECEIPT_BYTES + 1).unwrap();
    let mut errors = Vec::new();
    let mut total = 0;

    let result = read_claims(Some(&claims), &mut errors, &mut total);

    assert!(result.is_none());
    assert_eq!(total, 0);
    assert!(errors.join("; ").contains("exceeds"));
    let _ = fs::remove_file(claims);
}

#[test]
fn oversized_evidence_group_is_rejected_before_command_execution() {
    let evidence = (0..=MAX_EXECUTION_RECEIPTS)
        .map(|index| EvidenceSpec {
            evidence_id: format!("smoke.{index}"),
            evidence_ref: format!("receipts/smoke.{index}.json"),
            surface: "test".into(),
            mode: "unit".into(),
            target_os: std::env::consts::OS.into(),
            output: unique_output(&format!("receipt-{index}")),
            claims: None,
        })
        .collect::<Vec<_>>();
    let error = execute_and_write(
        &std::env::current_dir().unwrap(),
        &evidence,
        &ExecutionSpec {
            producer_id: "test.oversized".into(),
            producer_contract: "fnv1a64:1111111111111111".into(),
            command_artifacts: Vec::new(),
            producer_inputs: BTreeMap::new(),
            steps: vec![CommandStep {
                program: "deve-command-that-must-not-run".into(),
                args: Vec::new(),
                env: BTreeMap::new(),
            }],
            finally_steps: Vec::new(),
            timeout: Duration::from_secs(1),
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains("atomic evidence group exceeds"));
}

#[test]
fn one_failed_sibling_fails_the_complete_execution_group() {
    let mut receipts = vec![
        receipt_for_serialization("one", "passed", None),
        receipt_for_serialization("two", "failed", Some("claims missing")),
    ];

    let publications = serialize_receipts_bounded(&mut receipts).unwrap();

    assert_eq!(publications.len(), 2);
    assert!(
        receipts
            .iter()
            .all(|(_, _, receipt)| receipt.status == "failed")
    );
}

#[test]
fn aggregate_overflow_omits_claims_and_publishes_a_bounded_failed_group() {
    let mut receipts = (0..MAX_EXECUTION_RECEIPTS)
        .map(|index| {
            let mut receipt = receipt_for_serialization(&index.to_string(), "passed", None);
            receipt.2.claims = Some(serde_json::json!({ "payload": "x".repeat(300_000) }));
            receipt
        })
        .collect::<Vec<_>>();

    let publications = serialize_receipts_bounded(&mut receipts).unwrap();

    assert_eq!(publications.len(), MAX_EXECUTION_RECEIPTS);
    assert!(
        publications
            .iter()
            .map(|(_, bytes)| bytes.len())
            .sum::<usize>()
            < MAX_TOTAL_RECEIPT_BYTES as usize
    );
    assert!(
        receipts
            .iter()
            .all(|(_, _, receipt)| receipt.status == "failed" && receipt.claims.is_none())
    );
}

fn receipt_for_serialization(
    id: &str,
    status: &str,
    error: Option<&str>,
) -> (std::path::PathBuf, String, super::Receipt) {
    let evidence_id = format!("smoke.{id}");
    let evidence_ref = format!("receipts/{evidence_id}.json");
    (
        std::path::PathBuf::from(&evidence_ref),
        evidence_id.clone(),
        super::Receipt {
            schema: 3,
            producer_id: "test.group".into(),
            producer_contract: "fnv1a64:1111111111111111".into(),
            execution_id: "exec-fnv1a64-1111111111111111".into(),
            execution_evidence_ids: vec!["smoke.one".into(), "smoke.two".into()],
            evidence_id,
            evidence_ref,
            head: "abc".into(),
            head_after: Some("abc".into()),
            dirty_before: false,
            dirty_after: false,
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            target_os: "web".into(),
            surface: "web".into(),
            mode: "browser".into(),
            started_at: "2026-01-01T00:00:00Z".into(),
            finished_at: "2026-01-01T00:00:01Z".into(),
            status: status.into(),
            exit_code: Some(0),
            error: error.map(str::to_string),
            command_program: "test".into(),
            command_arg_count: 0,
            command_fingerprint: "fnv1a64:0123456789abcdef".into(),
            command_artifacts: Vec::new(),
            producer_inputs: BTreeMap::new(),
            claims: None,
        },
    )
}

fn receipt_args(output: &std::path::Path, file_name: &str, evidence_id: &str) -> Vec<String> {
    vec![
        "--evidence-id".into(),
        evidence_id.into(),
        "--evidence-ref".into(),
        file_name.into(),
        "--surface".into(),
        "test".into(),
        "--mode".into(),
        "unit".into(),
        "--target-os".into(),
        std::env::consts::OS.into(),
        "--output".into(),
        output.display().to_string(),
        "--".into(),
    ]
}

#[test]
fn one_execution_writes_multiple_evidence_bound_receipts() {
    let base = unique_output("multi-receipt-root").with_extension("");
    let repo = base.join("repo");
    let output = base.join("evidence");
    fs::create_dir_all(&repo).unwrap();
    for args in [
        vec!["init"],
        vec!["config", "user.email", "acceptance@example.invalid"],
        vec!["config", "user.name", "Acceptance Test"],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(&repo)
                .status()
                .unwrap()
                .success()
        );
    }
    fs::write(repo.join("tracked.txt"), "fixture").unwrap();
    assert!(
        Command::new("git")
            .args(["add", "."])
            .current_dir(&repo)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .args(["commit", "-m", "fixture"])
            .current_dir(&repo)
            .status()
            .unwrap()
            .success()
    );

    let evidence = ["one", "two"].map(|id| EvidenceSpec {
        evidence_id: format!("smoke.{id}"),
        evidence_ref: format!("receipts/smoke.{id}.json"),
        surface: "web".into(),
        mode: "browser".into(),
        target_os: "web".into(),
        output: output.join(format!("receipts/smoke.{id}.json")),
        claims: None,
    });
    execute_and_write(
        &repo,
        &evidence,
        &ExecutionSpec {
            producer_id: "test.multi".into(),
            producer_contract: "fnv1a64:1111111111111111".into(),
            command_artifacts: vec!["scripts/test.sh".into()],
            producer_inputs: BTreeMap::from([(
                "DEVE_TEST_CANDIDATE".into(),
                "sha256:fixture".into(),
            )]),
            steps: vec![CommandStep {
                program: "git".into(),
                args: vec!["--version".into()],
                env: BTreeMap::new(),
            }],
            finally_steps: Vec::new(),
            timeout: Duration::from_secs(10),
        },
    )
    .unwrap();
    let one: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output.join("receipts/smoke.one.json")).unwrap())
            .unwrap();
    let two: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output.join("receipts/smoke.two.json")).unwrap())
            .unwrap();
    assert_eq!(one["status"], "passed");
    assert_eq!(two["status"], "passed");
    assert_eq!(one["producer_id"], "test.multi");
    assert_eq!(one["execution_id"], two["execution_id"]);
    assert_eq!(
        one["execution_evidence_ids"],
        serde_json::json!(["smoke.one", "smoke.two"])
    );
    assert_eq!(one["command_fingerprint"], two["command_fingerprint"]);
    assert_eq!(
        one["producer_inputs"]["DEVE_TEST_CANDIDATE"],
        "sha256:fixture"
    );
    assert_ne!(one["evidence_id"], two["evidence_id"]);
    let _ = fs::remove_dir_all(base);
}
