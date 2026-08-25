//! Typed contracts for non-producer jobs required by the stable check fan-in.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::command::{require_node_24, require_rust_toolchain};
use super::yaml::{as_mapping, as_sequence, as_string, optional, required};
use anyhow::{Result, bail};
use yaml_rust2::Yaml;
use yaml_rust2::yaml::Hash;

const UBUNTU: &str = "ubuntu-latest";
const CHECK_ONLY_GUARD: &str = r#"forbidden=(
  "packages: ""write"
  "docker/""login-action"
  "docker/""metadata-action"
  "docker/""build-push-action"
  "push: ""true"
  "ghcr"".io"
  "tags: ""['v*']"
  "deve-acceptance-""receipts-"
  "deve-release-""candidate-"
)
for pattern in "${forbidden[@]}"; do
  if grep -nF "$pattern" .github/workflows/check.yml; then
    echo "check.yml must stay check-only: no package publish, Docker publish, release artifacts, registry publish, or tag release trigger."
    exit 1
  fi
done"#;
const ACCEPTANCE_PLANS: &str = r#"cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier ci --plan
cargo run --locked --quiet -p deve_baseline -- acceptance-run --tier tag-ready --plan"#;
const PLAN_COVERAGE: &str = r#"scripts/plan-coverage.sh --check-reverse-coverage
scripts/plan-coverage.sh --check-metadata-completeness
scripts/plan-coverage.sh --check-perf-budget
scripts/check-perf-budget-baseline.sh
scripts/check-reliability-observability-baseline.sh
scripts/plan-coverage.sh --check-no-adr-plan-ref
scripts/plan-coverage.sh --check-md-links docs/plan docs/features docs/acceptance-cases
scripts/plan-coverage-selftest.sh"#;
const REQUIRED_BASE_JOBS: [(&str, &[&str], bool); 3] = [
    (
        "contract-checks",
        &[
            "cargo fetch --locked",
            CHECK_ONLY_GUARD,
            "cargo fmt --check",
            "cargo run --locked --quiet -p deve_baseline -- all",
            ACCEPTANCE_PLANS,
            "node --test scripts/android-lifecycle-harness.test.mjs scripts/mobile-android-emulator-journey.test.mjs",
            PLAN_COVERAGE,
        ],
        true,
    ),
    (
        "rust-quality",
        &[
            "cargo fetch --locked",
            "cargo clippy --locked --all-targets -- -D warnings",
            "cargo check --locked -p deve_web --target wasm32-unknown-unknown",
        ],
        false,
    ),
    (
        "workspace-tests",
        &["cargo fetch --locked", "cargo test --locked"],
        false,
    ),
];
const WATCHER_COMMANDS: [&str; 4] = [
    "cargo fetch --locked",
    "cargo test --locked -p deve_core --test watcher_platform_fs -- --nocapture --test-threads=1",
    "cargo test --locked -p deve_core --test watcher_writeback_loop -- --nocapture --test-threads=1",
    "cargo test --locked -p deve_core --test watcher_rename_pairing -- --nocapture --test-threads=1",
];

pub(super) fn validate_required_base_jobs(jobs: &Hash) -> Result<()> {
    for (job_id, commands, needs_node) in REQUIRED_BASE_JOBS {
        let path = format!("check.yml.jobs.{job_id}");
        let job = as_mapping(required(jobs, job_id, "check.yml.jobs")?, &path)?;
        if as_string(required(job, "runs-on", &path)?, &format!("{path}.runs-on"))? != UBUNTU {
            bail!("acceptance producers: {path} must use the fixed Ubuntu host");
        }
        validate_job_commands(job, &path, commands, needs_node)?;
    }
    validate_watcher(jobs)
}

fn validate_job_commands(
    job: &Hash,
    path: &str,
    expected_commands: &[&str],
    needs_node: bool,
) -> Result<()> {
    let steps = as_sequence(required(job, "steps", path)?, &format!("{path}.steps"))?;
    let first_command_step = steps
        .iter()
        .position(|value| {
            value
                .as_hash()
                .is_some_and(|step| optional(step, "run").is_some())
        })
        .ok_or_else(|| anyhow::anyhow!("acceptance producers: {path} has no command steps"))?;
    require_rust_toolchain(steps, first_command_step, path)?;
    if needs_node {
        require_node_24(steps, first_command_step, path)?;
    }
    let commands = steps
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            value
                .as_hash()
                .and_then(|step| optional(step, "run").map(|run| (index, run)))
        })
        .map(|(index, run)| as_string(run, &format!("{path}.steps[{index}].run")).map(str::trim))
        .collect::<Result<Vec<_>>>()?;
    if commands != expected_commands {
        bail!(
            "acceptance producers: {path} command sequence must exactly match its required check-only contract"
        );
    }
    Ok(())
}

fn validate_watcher(jobs: &Hash) -> Result<()> {
    let job_id = "watcher-native-fs";
    let path = format!("check.yml.jobs.{job_id}");
    let job = as_mapping(required(jobs, job_id, "check.yml.jobs")?, &path)?;
    if as_string(required(job, "runs-on", &path)?, &format!("{path}.runs-on"))?
        != "${{ matrix.os }}"
    {
        bail!("acceptance producers: {path} must run on the exact watcher OS matrix");
    }
    let strategy_path = format!("{path}.strategy");
    let strategy = as_mapping(required(job, "strategy", &path)?, &strategy_path)?;
    if optional(strategy, "fail-fast") != Some(&Yaml::Boolean(false)) {
        bail!("acceptance producers: {strategy_path}.fail-fast must be false");
    }
    let matrix_path = format!("{strategy_path}.matrix");
    let matrix = as_mapping(required(strategy, "matrix", &strategy_path)?, &matrix_path)?;
    if matrix.len() != 1 {
        bail!("acceptance producers: {matrix_path} must contain only the OS axis");
    }
    let os = as_sequence(
        required(matrix, "os", &matrix_path)?,
        &format!("{matrix_path}.os"),
    )?
    .iter()
    .map(|value| as_string(value, &format!("{matrix_path}.os")))
    .collect::<Result<Vec<_>>>()?;
    if os != ["ubuntu-latest", "windows-latest"] {
        bail!("acceptance producers: {matrix_path}.os must contain exact Linux and Windows hosts");
    }
    validate_job_commands(job, &path, &WATCHER_COMMANDS, false)
}
