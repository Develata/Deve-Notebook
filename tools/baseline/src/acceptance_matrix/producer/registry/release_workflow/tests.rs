//! Candidate workflow projection regression tests.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::{collect_workflow, parse_candidate_command, validate_counts};
use crate::acceptance_matrix::producer::model::{Producer, ProducerRegistry};
use std::collections::BTreeMap;

fn producer(id: &str, candidate_required: bool) -> Producer {
    Producer {
        producer_id: id.into(),
        candidate_required,
        evidence_ids: vec![format!("evidence.{id}")],
        dependencies: Vec::new(),
        tiers: vec!["tag-ready".into()],
        host_os: vec!["linux".into()],
        timeout_seconds: 1,
        required_tools: Vec::new(),
        required_env: Vec::new(),
        bound_env: Vec::new(),
        environment: BTreeMap::new(),
        claims_env: BTreeMap::new(),
        artifacts: Vec::new(),
        steps: Vec::new(),
        finally_steps: Vec::new(),
        note: "fixture".into(),
    }
}

#[test]
fn candidate_command_parser_rejects_inert_or_duplicate_producers() {
    assert!(parse_candidate_command("echo acceptance-run", None, "fixture").is_err());
    assert!(
        parse_candidate_command(
            "cargo run --locked -p deve_baseline -- acceptance-run --tier tag-ready --producer one --producer one --receipt-dir /tmp/receipts",
            None,
            "fixture"
        )
        .is_err()
    );
    assert!(
        parse_candidate_command(
            "exit 0\ncargo run --locked -p deve_baseline -- acceptance-run --tier tag-ready --producer one --receipt-dir /tmp/receipts",
            None,
            "fixture"
        )
        .is_err()
    );
    assert!(
        parse_candidate_command(
            "cargo run --locked -p deve_baseline -- acceptance-run --tier tag-ready --plan --producer one --receipt-dir /tmp/receipts",
            None,
            "fixture"
        )
        .is_err()
    );
    assert!(
        parse_candidate_command(
            "if enabled; then\n  cargo run --locked -p deve_baseline -- acceptance-run --tier tag-ready --producer one --receipt-dir /tmp/receipts\nfi",
            None,
            "fixture"
        )
        .is_err()
    );
    assert!(
        parse_candidate_command(
            "cargo run --locked -p deve_baseline -- acceptance-run --tier tag-ready --producer one --receipt-dir /tmp/receipts || true",
            None,
            "fixture"
        )
        .is_err()
    );
    for suffix in ["||true", "&true", ";true", ">out", "$(true)", "`true`"] {
        let command = format!(
            "cargo run --locked -p deve_baseline -- acceptance-run --tier tag-ready --producer one --receipt-dir /tmp/receipts{suffix}"
        );
        assert!(parse_candidate_command(&command, None, "fixture").is_err());
    }
    assert!(
        parse_candidate_command(
            "cargo run --locked -p deve_baseline -- acceptance-run --tier tag-ready --producer one --receipt-dir /tmp/receipts",
            Some("echo {0}"),
            "fixture"
        )
        .is_err()
    );
    assert!(
        parse_candidate_command(
            "cat <<EOF\ncargo run --locked -p deve_baseline -- acceptance-run --tier tag-ready --producer one --receipt-dir /tmp/receipts\nEOF",
            None,
            "fixture"
        )
        .is_err()
    );
    assert!(
        parse_candidate_command(
            "cargo run --locked -p deve_baseline -- acceptance-run --tier tag-ready --producer one --receipt-dir /tmp/receipts\nexit 0",
            None,
            "fixture"
        )
        .is_err()
    );
    assert!(
        parse_candidate_command(
            "cargo run --locked -p deve_baseline -- acceptance-run --tier tag-ready --producer one --receipt-dir \"${{ runner.temp }}/receipts\"",
            Some("pwsh"),
            "fixture"
        )
        .is_ok()
    );
    assert!(
        parse_candidate_command(
            "true ||\ncargo run --locked -p deve_baseline -- acceptance-run --tier tag-ready --producer one --receipt-dir /tmp/receipts",
            None,
            "fixture"
        )
        .is_err()
    );
    assert!(
        parse_candidate_command(
            "\"$GITHUB_WORKSPACE/target/android-candidate-harness/debug/deve_baseline\" acceptance-run --tier target-host --producer android.local-backend --receipt-dir \"$RUNNER_TEMP/deve-acceptance-android-local\"",
            None,
            "fixture"
        )
        .is_ok()
    );
    assert!(
        parse_candidate_command(
            "/tmp/deve_baseline acceptance-run --tier target-host --producer android.local-backend --receipt-dir /tmp/receipts",
            None,
            "fixture"
        )
        .is_err()
    );
}

#[test]
fn workflow_parser_counts_only_unconditional_execution_steps() {
    let workflow = r#"
jobs:
  receipt:
    runs-on: ubuntu-latest
    timeout-minutes: 16
    steps:
      - run: cargo run --locked -p deve_baseline -- acceptance-run --tier tag-ready --producer test.one --receipt-dir /tmp/receipt
"#;
    let mut counts = BTreeMap::new();
    let timeouts = BTreeMap::from([("test.one", 1)]);
    collect_workflow("fixture.yml", workflow, &mut counts, &timeouts).unwrap();
    assert_eq!(counts.get("test.one"), Some(&1));

    let conditional = workflow.replace("      - run:", "      - if: success()\n        run:");
    assert!(
        collect_workflow("fixture.yml", &conditional, &mut BTreeMap::new(), &timeouts).is_err()
    );

    let matrix = workflow.replace(
        "    runs-on: ubuntu-latest",
        "    strategy:\n      matrix:\n        lane: [one, two]\n    runs-on: ubuntu-latest",
    );
    assert!(collect_workflow("fixture.yml", &matrix, &mut BTreeMap::new(), &timeouts).is_err());

    let short_budget = workflow.replace("timeout-minutes: 16", "timeout-minutes: 15");
    assert!(
        collect_workflow(
            "fixture.yml",
            &short_budget,
            &mut BTreeMap::new(),
            &timeouts
        )
        .is_err()
    );

    let workflow_defaults = format!("defaults:\n  run:\n    shell: echo {{0}}\n{workflow}");
    assert!(
        collect_workflow(
            "fixture.yml",
            &workflow_defaults,
            &mut BTreeMap::new(),
            &timeouts
        )
        .is_err()
    );
}

#[test]
fn candidate_projection_rejects_missing_extra_and_duplicate_producers() {
    let registry = ProducerRegistry {
        schema: 3,
        producers: vec![
            producer("candidate.one", true),
            producer("diagnostic.one", false),
        ],
    };
    assert!(validate_counts(&registry, &BTreeMap::new()).is_err());
    assert!(
        validate_counts(
            &registry,
            &BTreeMap::from([("candidate.one".into(), 1), ("diagnostic.one".into(), 1)])
        )
        .is_err()
    );
    assert!(validate_counts(&registry, &BTreeMap::from([("candidate.one".into(), 2)])).is_err());
    validate_counts(&registry, &BTreeMap::from([("candidate.one".into(), 1)])).unwrap();
}
