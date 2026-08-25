//! Adversarial coverage for the CI producer workflow projection.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::validate_text;
use crate::acceptance_matrix::producer::model::{
    Producer, ProducerArg, ProducerRegistry, ProducerStep,
};
use std::collections::BTreeMap;

const VALID: &str = r#"
jobs:
  core-checks:
    runs-on: ubuntu-latest
    steps:
      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --plan
      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier tag-ready --plan
  ci-acceptance-linux:
    runs-on: ubuntu-latest
    timeout-minutes: 40
    steps:
      - uses: dtolnay/rust-toolchain@fa04a1451ff1842e2626ccb99004d0195b455a88
        with:
          toolchain: "1.97.0"
      - uses: actions/setup-node@v6
        with:
          node-version: 24
      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --plan --producer ci.shared --producer ci.linux
      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.shared --producer ci.linux
  ci-acceptance-windows:
    runs-on: windows-latest
    timeout-minutes: 30
    steps:
      - uses: dtolnay/rust-toolchain@fa04a1451ff1842e2626ccb99004d0195b455a88
        with:
          toolchain: "1.97.0"
      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --plan --producer ci.windows
      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.windows
  watcher-native-fs:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test
  check:
    if: ${{ always() }}
    needs:
      - core-checks
      - ci-acceptance-linux
      - ci-acceptance-windows
      - watcher-native-fs
    runs-on: ubuntu-latest
    steps:
      - if: ${{ needs.core-checks.result != 'success' || needs.ci-acceptance-linux.result != 'success' || needs.ci-acceptance-windows.result != 'success' || needs.watcher-native-fs.result != 'success' }}
        run: exit 1
"#;

fn producer(producer_id: &str, host_os: &[&str], dependencies: &[&str], program: &str) -> Producer {
    Producer {
        producer_id: producer_id.to_owned(),
        evidence_ids: vec![format!("{producer_id}.evidence")],
        dependencies: dependencies
            .iter()
            .map(|dependency| (*dependency).to_owned())
            .collect(),
        tiers: vec!["ci".to_owned()],
        host_os: host_os.iter().map(|host| (*host).to_owned()).collect(),
        timeout_seconds: 600,
        required_env: Vec::new(),
        bound_env: Vec::new(),
        environment: BTreeMap::new(),
        claims_env: BTreeMap::new(),
        artifacts: Vec::new(),
        steps: vec![ProducerStep {
            program: program.to_owned(),
            args: vec![ProducerArg::LiteralString("--version".to_owned())],
        }],
        finally_steps: Vec::new(),
        note: String::new(),
    }
}

fn registry() -> ProducerRegistry {
    ProducerRegistry {
        schema: 1,
        producers: vec![
            producer("ci.shared", &["linux", "windows"], &[], "node"),
            producer("ci.linux", &["linux"], &["ci.shared"], "cargo"),
            producer("ci.windows", &["windows"], &[], "cargo"),
        ],
    }
}

fn error(workflow: &str) -> String {
    validate_text(workflow, &registry())
        .expect_err("workflow drift must fail closed")
        .to_string()
}

#[test]
fn accepts_exact_host_partition_and_fan_in() {
    validate_text(VALID, &registry()).expect("valid workflow partition");
}

#[test]
fn rejects_missing_duplicate_host_and_dependency_drift() {
    let missing = VALID.replace(
        " --producer ci.shared --producer ci.linux",
        " --producer ci.shared",
    );
    assert!(error(&missing).contains("exactly once"));

    let duplicate = VALID.replace(
        "      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --plan --producer ci.windows",
        "      - uses: actions/setup-node@v6\n        with:\n          node-version: 24\n      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --plan --producer ci.windows --producer ci.shared",
    ).replace(
        "      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.windows",
        "      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.windows --producer ci.shared",
    ).replacen("    timeout-minutes: 30", "    timeout-minutes: 40", 1);
    let duplicate_error = error(&duplicate);
    assert!(duplicate_error.contains("duplicate"), "{duplicate_error}");

    let wrong_host = VALID.replace("runs-on: windows-latest", "runs-on: ubuntu-latest");
    assert!(error(&wrong_host).contains("incompatible host"));

    let split_dependency = VALID.replace(
        " --producer ci.shared --producer ci.linux",
        " --producer ci.linux",
    );
    assert!(error(&split_dependency).contains("splits dependency"));
}

#[test]
fn rejects_inert_compound_conditional_and_tolerated_commands() {
    let echoed = VALID.replace(
        "run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.windows",
        "run: echo cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.windows",
    );
    assert!(error(&echoed).contains("canonical acceptance argv"));

    let compound = VALID.replace(
        "--tier ci --producer ci.windows",
        "--tier ci --producer ci.windows || true",
    );
    assert!(error(&compound).contains("expected --producer"));

    let conditional = VALID.replace(
        "      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.windows",
        "      - if: false\n        run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.windows",
    );
    assert!(error(&conditional).contains("may not declare if"));

    let tolerated = VALID.replace(
        "      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.windows",
        "      - continue-on-error: true\n        run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.windows",
    );
    assert!(error(&tolerated).contains("continue-on-error"));

    let name_only = VALID.replace(
        "      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.windows",
        "      - name: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.windows\n        run: true",
    );
    assert!(error(&name_only).contains("must be a string"));

    let multiline = VALID.replace(
        "run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.windows",
        "run: |\n          true\n          cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.windows",
    );
    assert!(error(&multiline).contains("inert or compound shell text"));
}

#[test]
fn rejects_missing_tools_short_deadline_and_plan_execution_drift() {
    let missing_rust = VALID.replacen(
        "      - uses: dtolnay/rust-toolchain@fa04a1451ff1842e2626ccb99004d0195b455a88\n        with:\n          toolchain: \"1.97.0\"\n",
        "",
        1,
    );
    assert!(error(&missing_rust).contains("exact Rust 1.97.0"));

    let missing_node = VALID.replace(
        "      - uses: actions/setup-node@v6\n        with:\n          node-version: 24\n",
        "",
    );
    assert!(error(&missing_node).contains("exact Node.js 24"));

    let short_deadline = VALID.replacen("timeout-minutes: 40", "timeout-minutes: 30", 1);
    assert!(error(&short_deadline).contains("producer deadline plus build margin"));

    let plan_drift = VALID.replacen(
        "--tier ci --plan --producer ci.shared --producer ci.linux",
        "--tier ci --plan --producer ci.shared",
        1,
    );
    assert!(error(&plan_drift).contains("producer sets differ"));
}

#[test]
fn rejects_job_expansion_and_execution_modifiers() {
    let workflow_defaults =
        VALID.replacen("jobs:", "defaults:\n  run:\n    shell: echo {0}\njobs:", 1);
    assert!(error(&workflow_defaults).contains("may not declare workflow defaults"));

    let workflow_env = VALID.replacen("jobs:", "env:\n  BASH_ENV: ./noop.sh\njobs:", 1);
    assert!(error(&workflow_env).contains("may not declare workflow env"));

    let matrix = VALID.replace(
        "    timeout-minutes: 40",
        "    timeout-minutes: 40\n    strategy:\n      matrix:\n        duplicate: [one, two]",
    );
    assert!(error(&matrix).contains("may not declare strategy"));

    let defaults = VALID.replace(
        "    timeout-minutes: 40",
        "    timeout-minutes: 40\n    defaults:\n      run:\n        shell: echo {0}",
    );
    assert!(error(&defaults).contains("may not declare defaults"));

    let custom_shell = VALID.replacen(
        "      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.shared --producer ci.linux",
        "      - shell: echo {0}\n        run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.shared --producer ci.linux",
        1,
    );
    assert!(error(&custom_shell).contains("may not declare shell"));

    let step_deadline = VALID.replacen(
        "      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.shared --producer ci.linux",
        "      - timeout-minutes: 1\n        run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.shared --producer ci.linux",
        1,
    );
    assert!(error(&step_deadline).contains("may not declare timeout-minutes"));

    let step_env = VALID.replacen(
        "      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.shared --producer ci.linux",
        "      - env:\n          RUSTC_WRAPPER: true\n        run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.shared --producer ci.linux",
        1,
    );
    assert!(error(&step_env).contains("may not declare env"));

    let working_directory = VALID.replacen(
        "      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.shared --producer ci.linux",
        "      - working-directory: fixtures/other-workspace\n        run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.shared --producer ci.linux",
        1,
    );
    assert!(error(&working_directory).contains("may not declare working-directory"));

    let conditional_setup = VALID.replacen(
        "      - uses: dtolnay/rust-toolchain@fa04a1451ff1842e2626ccb99004d0195b455a88",
        "      - if: false\n        uses: dtolnay/rust-toolchain@fa04a1451ff1842e2626ccb99004d0195b455a88",
        1,
    );
    assert!(error(&conditional_setup).contains("may not declare if"));

    let wrong_setup_action = VALID.replacen(
        "dtolnay/rust-toolchain@fa04a1451ff1842e2626ccb99004d0195b455a88",
        "dtolnay/rust-toolchain@master",
        1,
    );
    assert!(error(&wrong_setup_action).contains("must install exact Rust"));

    let rust_setup = "      - uses: dtolnay/rust-toolchain@fa04a1451ff1842e2626ccb99004d0195b455a88\n        with:\n          toolchain: \"1.97.0\"\n";
    let late_setup = VALID.replacen(rust_setup, "", 1).replacen(
        "      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.shared --producer ci.linux",
        "      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --producer ci.shared --producer ci.linux\n      - uses: dtolnay/rust-toolchain@fa04a1451ff1842e2626ccb99004d0195b455a88\n        with:\n          toolchain: \"1.97.0\"",
        1,
    );
    assert!(error(&late_setup).contains("before producer commands"));

    let unlocked = VALID.replacen("cargo run --locked", "cargo run", 1);
    assert!(error(&unlocked).contains("expected --locked"));

    let mut cleanup_registry = registry();
    cleanup_registry.producers[0]
        .finally_steps
        .push(ProducerStep {
            program: "cargo".to_owned(),
            args: Vec::new(),
        });
    let cleanup_deadline = VALID.replacen("timeout-minutes: 40", "timeout-minutes: 35", 1);
    let cleanup_error = validate_text(&cleanup_deadline, &cleanup_registry)
        .expect_err("cleanup deadline must be included")
        .to_string();
    assert!(cleanup_error.contains("producer deadline plus build margin"));
}

#[test]
fn rejects_fan_in_that_can_hide_non_success() {
    let conditional = VALID.replace("if: ${{ always() }}", "if: false");
    assert!(error(&conditional).contains("must be a string"));

    let missing_need = VALID.replace("      - watcher-native-fs\n", "");
    assert!(error(&missing_need).contains("needs mismatch"));

    let weak_condition = VALID.replace(
        "needs.watcher-native-fs.result != 'success'",
        "needs.watcher-native-fs.result == 'failure'",
    );
    assert!(error(&weak_condition).contains("does not reject every non-success"));

    let inert = VALID.replace("        run: exit 1", "        run: true");
    assert!(error(&inert).contains("must be a string"));

    let tolerated = VALID.replace(
        "        run: exit 1",
        "        continue-on-error: true\n        run: exit 1",
    );
    let tolerated_error = error(&tolerated);
    assert!(
        tolerated_error.contains("may not tolerate fan-in failure"),
        "{tolerated_error}"
    );

    let custom_shell = VALID.replace(
        "        run: exit 1",
        "        shell: echo {0}\n        run: exit 1",
    );
    assert!(error(&custom_shell).contains("may not declare shell"));

    let fan_in_defaults = VALID.replace(
        "  check:\n    if: ${{ always() }}",
        "  check:\n    defaults:\n      run:\n        shell: echo {0}\n    if: ${{ always() }}",
    );
    assert!(error(&fan_in_defaults).contains("may not declare defaults"));

    let tolerated_dependency = VALID.replace(
        "  core-checks:\n    runs-on: ubuntu-latest",
        "  core-checks:\n    continue-on-error: true\n    runs-on: ubuntu-latest",
    );
    assert!(error(&tolerated_dependency).contains("required fan-in dependency failure"));

    let skipped_dependency_step = VALID.replacen(
        "      - run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --plan",
        "      - if: false\n        run: cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --plan",
        1,
    );
    assert!(error(&skipped_dependency_step).contains("may not declare if"));
}
