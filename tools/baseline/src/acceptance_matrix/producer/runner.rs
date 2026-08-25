//! Side-effecting producer execution and host tool resolution.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::execution_policy::FINALLY_STEP_TIMEOUT_SECONDS;
use super::model::{Producer, ProducerArg, ProducerStep};
use super::plan::{ProducerPlan, git_output, git_status};
use super::registry;
use crate::acceptance_matrix::receipt::{
    CommandStep, EvidenceSpec, ExecutionSpec, execute_and_write, run_step,
};
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(super) fn run_producer(root: &Path, receipt_dir: &Path, plan: &ProducerPlan<'_>) -> Result<()> {
    with_producer_duration(&plan.producer.producer_id, || {
        run_producer_inner(root, receipt_dir, plan)
    })
}

fn run_producer_inner(root: &Path, receipt_dir: &Path, plan: &ProducerPlan<'_>) -> Result<()> {
    let claims_root = receipt_dir.join("claims").join(&plan.producer.producer_id);
    let state_root = receipt_dir.join("state").join(&plan.producer.producer_id);
    let (mut environment, producer_inputs) = producer_environment(plan.producer, &state_root)?;
    let mut evidence = Vec::new();
    for row in &plan.evidence {
        let claims = plan.producer.claims_env.get(&row.evidence_id).map(|name| {
            let path = claims_root.join(format!("{}.claims", row.evidence_id));
            environment.insert(name.clone(), path.display().to_string());
            path
        });
        evidence.push(EvidenceSpec {
            evidence_id: row.evidence_id.clone(),
            evidence_ref: row.evidence_ref.clone(),
            surface: row.surface.clone(),
            mode: row.mode.clone(),
            target_os: expected_target_os(&row.surface).to_string(),
            output: receipt_dir.join(Path::new(&row.evidence_ref)),
            claims,
        });
    }
    if !plan.producer.claims_env.is_empty() {
        fs::create_dir_all(&claims_root).with_context(|| {
            format!(
                "acceptance-run: failed to create claims directory {}",
                claims_root.display()
            )
        })?;
    }
    let execution_spec = execution_spec(plan.producer, &environment, producer_inputs)?;
    println!(
        "acceptance-run: running {} ({})",
        plan.producer.producer_id, plan.producer.note
    );
    execute_and_write(root, &evidence, &execution_spec)
}

pub(super) fn run_static_producer(
    root: &Path,
    state_parent: &Path,
    plan: &ProducerPlan<'_>,
) -> Result<()> {
    with_producer_duration(&plan.producer.producer_id, || {
        run_static_producer_inner(root, state_parent, plan)
    })
}

fn run_static_producer_inner(
    root: &Path,
    state_parent: &Path,
    plan: &ProducerPlan<'_>,
) -> Result<()> {
    let state_root = state_parent.join(&plan.producer.producer_id);
    let (environment, producer_inputs) = producer_environment(plan.producer, &state_root)?;
    let execution_spec = execution_spec(plan.producer, &environment, producer_inputs)?;
    let head = git_output(root, ["rev-parse", "HEAD"])?;
    let execution_started = Instant::now();
    println!(
        "acceptance-run: running {} ({})",
        plan.producer.producer_id, plan.producer.note
    );
    let mut primary_error = None;
    for step in &execution_spec.steps {
        let remaining = execution_spec
            .timeout
            .saturating_sub(execution_started.elapsed());
        if remaining.is_zero() {
            primary_error = Some(anyhow::anyhow!(
                "acceptance-run: producer {} timed out before the next step",
                plan.producer.producer_id
            ));
            break;
        }
        match run_step(root, step, remaining) {
            Ok(status) if status.success() => {}
            Ok(_) => {
                primary_error = Some(anyhow::anyhow!(
                    "acceptance-run: producer {} command {} returned non-zero",
                    plan.producer.producer_id,
                    step.program
                ));
                break;
            }
            Err(error) => {
                primary_error = Some(error.context(format!(
                    "acceptance-run: producer {} failed",
                    plan.producer.producer_id
                )));
                break;
            }
        }
    }
    let mut cleanup_errors = Vec::new();
    for step in &execution_spec.finally_steps {
        match run_step(
            root,
            step,
            Duration::from_secs(FINALLY_STEP_TIMEOUT_SECONDS),
        ) {
            Ok(status) if status.success() => {}
            Ok(_) => {
                cleanup_errors.push(format!("cleanup step {} returned non-zero", step.program))
            }
            Err(error) => {
                cleanup_errors.push(format!("cleanup step {} failed: {error}", step.program))
            }
        }
    }
    let head_after = git_output(root, ["rev-parse", "HEAD"])?;
    let dirty_after = git_status(root)?;
    if head_after != head || !dirty_after.is_empty() {
        bail!(
            "acceptance-run: producer {} changed HEAD or dirtied the worktree",
            plan.producer.producer_id
        );
    }
    if !cleanup_errors.is_empty() {
        bail!(
            "acceptance-run: producer {} cleanup failed: {}",
            plan.producer.producer_id,
            cleanup_errors.join("; ")
        );
    }
    if let Some(error) = primary_error {
        return Err(error);
    }
    Ok(())
}

fn with_producer_duration<T>(
    producer_id: &str,
    operation: impl FnOnce() -> Result<T>,
) -> Result<T> {
    let started = Instant::now();
    let result = operation();
    report_producer_duration(producer_id, result.is_ok(), started.elapsed());
    result
}

fn report_producer_duration(producer_id: &str, success: bool, duration: Duration) {
    let diagnostic = producer_duration_diagnostic(producer_id, success, duration);
    let _ = writeln!(io::stdout().lock(), "{diagnostic}");
}

fn producer_duration_diagnostic(producer_id: &str, success: bool, duration: Duration) -> String {
    let status = if success { "ok" } else { "failed" };
    format!(
        "acceptance-run: producer {producer_id} status={status} duration_ms={}",
        duration.as_millis()
    )
}

fn producer_environment(
    producer: &Producer,
    state_root: &Path,
) -> Result<(BTreeMap<String, String>, BTreeMap<String, String>)> {
    let mut environment = producer.environment.clone();
    let baseline_executable = std::env::current_exe()
        .context("acceptance-run: failed to resolve the current baseline executable")?;
    environment.insert(
        "DEVE_BASELINE_BIN".into(),
        baseline_executable.display().to_string(),
    );
    fs::create_dir_all(state_root).with_context(|| {
        format!(
            "acceptance-run: failed to create producer state directory {}",
            state_root.display()
        )
    })?;
    environment.insert(
        "DEVE_ACCEPTANCE_PRODUCER_STATE_DIR".into(),
        state_root.to_string_lossy().replace('\\', "/"),
    );
    for name in &producer.required_env {
        let value = std::env::var(name).with_context(|| {
            format!(
                "acceptance-run: producer {} missing required environment {name}",
                producer.producer_id
            )
        })?;
        environment.insert(name.clone(), value);
    }
    let producer_inputs = producer
        .bound_env
        .iter()
        .map(|name| {
            environment
                .get(name)
                .cloned()
                .map(|value| (name.clone(), value))
                .with_context(|| {
                    format!(
                        "acceptance-run: producer {} missing bound environment {name}",
                        producer.producer_id
                    )
                })
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    Ok((environment, producer_inputs))
}

fn execution_spec(
    producer: &Producer,
    environment: &BTreeMap<String, String>,
    producer_inputs: BTreeMap<String, String>,
) -> Result<ExecutionSpec> {
    Ok(ExecutionSpec {
        producer_id: producer.producer_id.clone(),
        producer_contract: registry::contract_fingerprint(producer)?,
        command_artifacts: producer.artifacts.clone(),
        producer_inputs,
        steps: resolve_steps(producer, &producer.steps, environment)?,
        finally_steps: resolve_steps(producer, &producer.finally_steps, environment)?,
        timeout: Duration::from_secs(producer.timeout_seconds),
    })
}

pub(super) fn staging_directory(output: &Path) -> Result<PathBuf> {
    let parent = output
        .parent()
        .context("acceptance-run: receipt directory has no parent")?;
    fs::create_dir_all(parent)?;
    let name = output
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("receipts");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("acceptance-run: system clock predates UNIX epoch")?
        .as_nanos();
    let staging = parent.join(format!(".{name}.run-{}-{unique}", std::process::id()));
    if staging.exists() {
        bail!(
            "acceptance-run: staging directory already exists: {}",
            staging.display()
        );
    }
    Ok(staging)
}

fn resolve_steps(
    producer: &Producer,
    steps: &[ProducerStep],
    environment: &BTreeMap<String, String>,
) -> Result<Vec<CommandStep>> {
    steps
        .iter()
        .map(|step| {
            let args = step
                .args
                .iter()
                .map(|argument| match argument {
                    ProducerArg::LiteralString(literal) => Ok(literal.clone()),
                    ProducerArg::Literal { literal } => Ok(literal.clone()),
                    ProducerArg::Env { env } => environment.get(env).cloned().with_context(|| {
                        format!(
                            "acceptance-run: producer {} missing argument env {env}",
                            producer.producer_id
                        )
                    }),
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(CommandStep {
                program: resolve_program(&step.program)?,
                args,
                env: environment.clone(),
            })
        })
        .collect()
}

fn resolve_program(program: &str) -> Result<String> {
    #[cfg(windows)]
    if program == "bash" {
        if let Some(configured) = std::env::var_os("DEVE_ACCEPTANCE_BASH") {
            let path = PathBuf::from(configured);
            if path.is_file() {
                return Ok(path.display().to_string());
            }
            bail!(
                "acceptance-run: DEVE_ACCEPTANCE_BASH is not a file: {}",
                path.display()
            );
        }
        if let Ok(output) = Command::new("git").arg("--exec-path").output()
            && output.status.success()
            && let Ok(exec_path) = String::from_utf8(output.stdout)
        {
            for candidate in git_bash_candidates(Path::new(exec_path.trim())) {
                if candidate.is_file() {
                    return fs::canonicalize(&candidate)
                        .with_context(|| {
                            format!(
                                "acceptance-run: failed to canonicalize Git Bash at {}",
                                candidate.display()
                            )
                        })
                        .map(|path| path.display().to_string());
                }
            }
        }
        for candidate in [
            PathBuf::from(r"C:\Program Files\Git\bin\bash.exe"),
            PathBuf::from(r"C:\Program Files (x86)\Git\bin\bash.exe"),
        ] {
            if candidate.is_file() {
                return Ok(candidate.display().to_string());
            }
        }
        bail!(
            "acceptance-run: Git Bash is required on Windows; set DEVE_ACCEPTANCE_BASH to bash.exe"
        );
    }
    Ok(program.to_string())
}

#[cfg(windows)]
fn git_bash_candidates(exec_path: &Path) -> Vec<PathBuf> {
    exec_path
        .ancestors()
        .take(8)
        .flat_map(|ancestor| {
            [
                ancestor.join("bin").join("bash.exe"),
                ancestor.join("usr").join("bin").join("bash.exe"),
            ]
        })
        .collect()
}

fn expected_target_os(surface: &str) -> &str {
    match surface {
        "web" => "web",
        "docker" => "linux",
        "desktop" => "windows",
        "android" => "android",
        "release" => "multi-platform",
        "github" => "github",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::{producer_duration_diagnostic, with_producer_duration};
    use std::time::Duration;

    #[cfg(windows)]
    use super::git_bash_candidates;

    #[cfg(windows)]
    #[test]
    fn git_bash_candidates_include_git_install_root() {
        let candidates = git_bash_candidates(std::path::Path::new(
            r"C:\tools\git\mingw64\libexec\git-core",
        ));
        assert!(candidates.contains(&std::path::PathBuf::from(r"C:\tools\git\bin\bash.exe")));
    }

    #[test]
    fn producer_duration_diagnostic_is_fixed_and_secret_free() {
        assert_eq!(
            producer_duration_diagnostic(
                "ci.storage-repository-cases",
                true,
                Duration::from_millis(1234),
            ),
            "acceptance-run: producer ci.storage-repository-cases status=ok duration_ms=1234"
        );
        assert_eq!(
            producer_duration_diagnostic("ci.failed", false, Duration::from_millis(9)),
            "acceptance-run: producer ci.failed status=failed duration_ms=9"
        );
    }

    #[test]
    fn duration_wrapper_preserves_the_primary_error() {
        let error = with_producer_duration("ci.failed", || -> anyhow::Result<()> {
            anyhow::bail!("primary producer failure")
        })
        .expect_err("operation must remain failed")
        .to_string();
        assert_eq!(error, "primary producer failure");
    }
}
