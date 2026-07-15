//! Side-effecting producer execution and host tool resolution.
//! plan_ref: 18_release#first-tag-acceptance-matrix

use super::model::{Producer, ProducerArg, ProducerStep};
use super::plan::ProducerPlan;
use super::registry;
use crate::acceptance_matrix::receipt::{
    CommandStep, EvidenceSpec, ExecutionSpec, execute_and_write,
};
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(super) fn run_producer(root: &Path, receipt_dir: &Path, plan: &ProducerPlan<'_>) -> Result<()> {
    let claims_root = receipt_dir.join("claims").join(&plan.producer.producer_id);
    let state_root = receipt_dir.join("state").join(&plan.producer.producer_id);
    let mut environment = plan.producer.environment.clone();
    fs::create_dir_all(&state_root).with_context(|| {
        format!(
            "acceptance-run: failed to create producer state directory {}",
            state_root.display()
        )
    })?;
    environment.insert(
        "DEVE_ACCEPTANCE_PRODUCER_STATE_DIR".into(),
        state_root.to_string_lossy().replace('\\', "/"),
    );
    for name in &plan.producer.required_env {
        let value = std::env::var(name).with_context(|| {
            format!(
                "acceptance-run: producer {} missing required environment {name}",
                plan.producer.producer_id
            )
        })?;
        environment.insert(name.clone(), value);
    }
    let producer_inputs = plan
        .producer
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
                        plan.producer.producer_id
                    )
                })
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
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
    let steps = resolve_steps(plan.producer, &plan.producer.steps, &environment)?;
    let finally_steps = resolve_steps(plan.producer, &plan.producer.finally_steps, &environment)?;
    println!(
        "acceptance-run: running {} ({})",
        plan.producer.producer_id, plan.producer.note
    );
    execute_and_write(
        root,
        &evidence,
        &ExecutionSpec {
            producer_id: plan.producer.producer_id.clone(),
            producer_contract: registry::contract_fingerprint(plan.producer)?,
            command_artifacts: plan.producer.artifacts.clone(),
            producer_inputs,
            steps,
            finally_steps,
            timeout: Duration::from_secs(plan.producer.timeout_seconds),
        },
    )
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
}
