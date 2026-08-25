//! Stable required-check fan-in validation.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::base_jobs::validate_required_base_jobs;
use super::cache::validate_required_job_cache;
use super::command::{
    reject_job_execution_modifiers, reject_step_execution_modifiers,
    reject_tolerated_or_conditional,
};
use super::yaml::{as_mapping, as_sequence, as_string, optional, required, scalar_text};
use crate::release::toolchain::RUST_ACTION_REF;
use anyhow::{Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use yaml_rust2::{Yaml, yaml::Hash};

const FAN_IN_JOB: &str = "check";
const SHADOW_JOB: &str = "impact-shadow";
const FAN_IN_IF: &str = "${{ always() }}";
const BASE_REQUIRED_JOBS: [&str; 3] = ["contract-checks", "rust-quality", "workspace-tests"];
const SHADOW_NON_PR_RUN: &str = r#"set -euo pipefail
mkdir -p target/acceptance-impact
cargo run --locked --quiet -p deve_baseline -- acceptance-impact \
  --profile main-full-source >target/acceptance-impact/full-source-plan.json"#;

pub(super) fn validate_fan_in(jobs: &Hash, execution_jobs: &BTreeSet<String>) -> Result<()> {
    let mut expected_jobs = BASE_REQUIRED_JOBS
        .map(str::to_owned)
        .into_iter()
        .collect::<BTreeSet<_>>();
    expected_jobs.extend(execution_jobs.iter().cloned());
    expected_jobs.extend([
        "watcher-native-fs".to_owned(),
        SHADOW_JOB.to_owned(),
        FAN_IN_JOB.to_owned(),
    ]);
    let actual_jobs = jobs
        .keys()
        .map(|key| as_string(key, "check.yml.jobs key").map(str::to_owned))
        .collect::<Result<BTreeSet<_>>>()?;
    if actual_jobs != expected_jobs {
        bail!(
            "acceptance producers: check.yml job set mismatch; expected={expected_jobs:?} actual={actual_jobs:?}"
        );
    }
    let path = format!("check.yml.jobs.{FAN_IN_JOB}");
    if execution_jobs.contains(FAN_IN_JOB) {
        bail!("acceptance producers: stable check fan-in may not execute producers");
    }
    let job = as_mapping(required(jobs, FAN_IN_JOB, "check.yml.jobs")?, &path)?;
    reject_job_execution_modifiers(job, &path)?;
    if as_string(required(job, "if", &path)?, &format!("{path}.if"))? != FAN_IN_IF {
        bail!("acceptance producers: {path} must run under exact always() fan-in policy");
    }
    if optional(job, "continue-on-error").is_some() {
        bail!("acceptance producers: {path} may not tolerate fan-in failure");
    }
    if as_string(required(job, "runs-on", &path)?, &format!("{path}.runs-on"))? != "ubuntu-latest" {
        bail!("acceptance producers: {path} must use the fixed Ubuntu host");
    }
    let mut ordered_needs = BASE_REQUIRED_JOBS.map(str::to_owned).to_vec();
    ordered_needs.extend(execution_jobs.iter().cloned());
    ordered_needs.push("watcher-native-fs".to_owned());
    ordered_needs.push(SHADOW_JOB.to_owned());
    let expected_needs = ordered_needs.iter().cloned().collect::<BTreeSet<_>>();
    let need_values = as_sequence(required(job, "needs", &path)?, &format!("{path}.needs"))?;
    let needs = need_values
        .iter()
        .map(|value| as_string(value, &format!("{path}.needs")))
        .collect::<Result<BTreeSet<_>>>()?
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if needs.len() != need_values.len() {
        bail!("acceptance producers: {path}.needs may not contain duplicates");
    }
    if needs != expected_needs {
        bail!(
            "acceptance producers: {path} needs mismatch; expected={expected_needs:?} actual={needs:?}"
        );
    }
    for dependency in &needs {
        if dependency == SHADOW_JOB {
            continue;
        }
        let dependency_path = format!("check.yml.jobs.{dependency}");
        let dependency_job = as_mapping(
            required(jobs, dependency, "check.yml.jobs")?,
            &dependency_path,
        )?;
        if optional(dependency_job, "continue-on-error").is_some() {
            bail!(
                "acceptance producers: {dependency_path} may not tolerate a required fan-in dependency failure"
            );
        }
        for key in ["defaults", "env", "container", "services"] {
            if optional(dependency_job, key).is_some() {
                bail!(
                    "acceptance producers: {dependency_path} may not declare {key}; required fan-in dependencies must retain canonical execution semantics"
                );
            }
        }
        let dependency_steps = as_sequence(
            required(dependency_job, "steps", &dependency_path)?,
            &format!("{dependency_path}.steps"),
        )?;
        validate_required_job_cache(dependency_steps, &dependency_path)?;
        for (index, value) in dependency_steps.iter().enumerate() {
            let step_path = format!("{dependency_path}.steps[{index}]");
            let dependency_step = as_mapping(value, &step_path)?;
            if optional(dependency_step, "run").is_none() {
                continue;
            }
            reject_tolerated_or_conditional(dependency_step, &step_path)?;
            reject_step_execution_modifiers(dependency_step, &step_path)?;
        }
    }
    validate_shadow_job(jobs, execution_jobs)?;
    validate_required_base_jobs(jobs)?;
    let steps = as_sequence(required(job, "steps", &path)?, &format!("{path}.steps"))?;
    if steps.len() != 1 {
        bail!("acceptance producers: {path} must contain one exact failure step");
    }
    let step_path = format!("{path}.steps[0]");
    let step = as_mapping(&steps[0], &step_path)?;
    reject_step_execution_modifiers(step, &step_path)?;
    let expected_failure_if = format!(
        "${{{{ {} }}}}",
        ordered_needs
            .iter()
            .map(|dependency| format!("needs.{dependency}.result != 'success'"))
            .collect::<Vec<_>>()
            .join(" || ")
    );
    if as_string(
        required(step, "if", &step_path)?,
        &format!("{step_path}.if"),
    )? != expected_failure_if
    {
        bail!("acceptance producers: {step_path} does not reject every non-success result");
    }
    if as_string(
        required(step, "run", &step_path)?,
        &format!("{step_path}.run"),
    )?
    .trim()
        != "exit 1"
    {
        bail!("acceptance producers: {step_path} must fail with exact `exit 1`");
    }
    if optional(step, "continue-on-error").is_some() {
        bail!("acceptance producers: {step_path} may not tolerate fan-in failure");
    }
    Ok(())
}

fn validate_shadow_job(jobs: &Hash, execution_jobs: &BTreeSet<String>) -> Result<()> {
    let path = format!("check.yml.jobs.{SHADOW_JOB}");
    let job = as_mapping(required(jobs, SHADOW_JOB, "check.yml.jobs")?, &path)?;
    if as_string(required(job, "if", &path)?, &format!("{path}.if"))? != FAN_IN_IF
        || optional(job, "continue-on-error").is_some()
        || as_string(required(job, "runs-on", &path)?, &format!("{path}.runs-on"))?
            != "ubuntu-latest"
    {
        bail!("acceptance producers: {path} must be an untolerated always() Ubuntu audit job");
    }
    reject_job_execution_modifiers(job, &path)?;
    if job
        .get(&Yaml::String("timeout-minutes".into()))
        .and_then(Yaml::as_i64)
        != Some(30)
    {
        bail!("acceptance producers: {path}.timeout-minutes must equal 30");
    }
    let mut expected_needs = BASE_REQUIRED_JOBS
        .map(str::to_owned)
        .into_iter()
        .collect::<BTreeSet<_>>();
    expected_needs.extend(execution_jobs.iter().cloned());
    expected_needs.insert("watcher-native-fs".into());
    let need_values = as_sequence(required(job, "needs", &path)?, &format!("{path}.needs"))?;
    let actual_needs = need_values
        .iter()
        .map(|value| as_string(value, &format!("{path}.needs")))
        .collect::<Result<BTreeSet<_>>>()?
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if actual_needs.len() != need_values.len() {
        bail!("acceptance producers: {path}.needs may not contain duplicates");
    }
    if actual_needs != expected_needs {
        bail!("acceptance producers: {path} must observe every full CI job exactly once");
    }
    let steps = as_sequence(required(job, "steps", &path)?, &format!("{path}.steps"))?;
    if steps.len() != 7 {
        bail!("acceptance producers: {path} must contain seven exact audit steps");
    }
    validate_shadow_setup_steps(steps, &path)?;
    validate_shadow_report_step(&steps[4], true, &path, &expected_needs)?;
    validate_shadow_report_step(&steps[5], false, &path, &expected_needs)?;
    validate_shadow_upload_step(&steps[6], &path)?;
    Ok(())
}

fn validate_shadow_setup_steps(steps: &[Yaml], job_path: &str) -> Result<()> {
    let checkout = as_mapping(&steps[0], &format!("{job_path}.steps[0]"))?;
    require_exact_step_string(checkout, "uses", "actions/checkout@v6", job_path, 0)?;
    let checkout_with = as_mapping(
        required(checkout, "with", &format!("{job_path}.steps[0]"))?,
        &format!("{job_path}.steps[0].with"),
    )?;
    if checkout_with.len() != 1
        || scalar_text(required(
            checkout_with,
            "fetch-depth",
            &format!("{job_path}.steps[0].with"),
        )?)? != "0"
    {
        bail!("acceptance producers: {job_path}.steps[0] must fetch complete history");
    }
    let rust = as_mapping(&steps[1], &format!("{job_path}.steps[1]"))?;
    require_exact_step_string(rust, "uses", RUST_ACTION_REF, job_path, 1)?;
    let toolchain = as_mapping(
        required(rust, "with", &format!("{job_path}.steps[1]"))?,
        &format!("{job_path}.steps[1].with"),
    )?;
    if toolchain.len() != 1
        || scalar_text(required(
            toolchain,
            "toolchain",
            &format!("{job_path}.steps[1].with"),
        )?)? != "1.97.0"
    {
        bail!("acceptance producers: {job_path}.steps[1] must install exact Rust 1.97.0");
    }
    // The source cache contract applies only to the immutable setup prefix.
    // The final upload is separately validated below and must not be admitted
    // as a general-purpose action for required execution jobs.
    validate_required_job_cache(&steps[..4], job_path)?;
    let fetch = as_mapping(&steps[3], &format!("{job_path}.steps[3]"))?;
    require_exact_step_string(fetch, "run", "cargo fetch --locked", job_path, 3)?;
    forbid_step_modifiers(fetch, job_path, 3, false)?;
    Ok(())
}

fn validate_shadow_report_step(
    value: &Yaml,
    pull_request: bool,
    job_path: &str,
    full_jobs: &BTreeSet<String>,
) -> Result<()> {
    let index = if pull_request { 4 } else { 5 };
    let step_path = format!("{job_path}.steps[{index}]");
    let step = as_mapping(value, &step_path)?;
    let expected_if = if pull_request {
        "${{ github.event_name == 'pull_request' }}"
    } else {
        "${{ github.event_name != 'pull_request' }}"
    };
    require_exact_step_string(step, "if", expected_if, job_path, index)?;
    forbid_step_modifiers(step, job_path, index, pull_request)?;
    if pull_request {
        let env = as_mapping(
            required(step, "env", &step_path)?,
            &format!("{step_path}.env"),
        )?;
        if env.len() != full_jobs.len() + 2 {
            bail!("acceptance producers: {step_path}.env must bind every full CI result exactly");
        }
        if as_string(
            required(env, "BASE_SHA", &format!("{step_path}.env"))?,
            &format!("{step_path}.env.BASE_SHA"),
        )? != "${{ github.event.pull_request.base.sha }}"
            || as_string(
                required(env, "HEAD_SHA", &format!("{step_path}.env"))?,
                &format!("{step_path}.env.HEAD_SHA"),
            )? != "${{ github.sha }}"
        {
            bail!("acceptance producers: {step_path}.env must bind exact PR revisions");
        }
        let mut variable_by_job = BTreeMap::new();
        for (key, value) in env {
            let key = as_string(key, &format!("{step_path}.env key"))?;
            if matches!(key, "BASE_SHA" | "HEAD_SHA") {
                continue;
            }
            if !key.starts_with("RESULT_")
                || !key
                    .chars()
                    .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
            {
                bail!("acceptance producers: {step_path}.env result key is not canonical");
            }
            let value = as_string(value, &format!("{step_path}.env.{key}"))?;
            let (job, expression) = value.split_once('=').ok_or_else(|| {
                anyhow::anyhow!("acceptance producers: {step_path}.env.{key} must be job=result")
            })?;
            let expected = format!("${{{{ needs.{job}.result }}}}");
            if !full_jobs.contains(job) || expression != expected {
                bail!(
                    "acceptance producers: {step_path}.env.{key} does not bind a required job result"
                );
            }
            if variable_by_job
                .insert(job.to_owned(), key.to_owned())
                .is_some()
            {
                bail!("acceptance producers: {step_path}.env duplicates a required job result");
            }
        }
        if variable_by_job.keys().collect::<BTreeSet<_>>()
            != full_jobs.iter().collect::<BTreeSet<_>>()
        {
            bail!("acceptance producers: {step_path}.env omits a required job result");
        }
        let expected_run = shadow_pr_run(&variable_by_job);
        require_exact_step_string(step, "run", &expected_run, job_path, index)?;
    } else if optional(step, "env").is_some() {
        bail!("acceptance producers: {step_path} may not declare env");
    } else {
        require_exact_step_string(step, "run", SHADOW_NON_PR_RUN, job_path, index)?;
    }
    Ok(())
}

fn shadow_pr_run(variable_by_job: &BTreeMap<String, String>) -> String {
    let mut lines = vec![
        "set -euo pipefail".to_owned(),
        "mkdir -p target/acceptance-impact".to_owned(),
        "cargo run --locked --quiet -p deve_baseline -- acceptance-impact-shadow \\".to_owned(),
        "  --base \"$BASE_SHA\" \\".to_owned(),
        "  --head \"$HEAD_SHA\" \\".to_owned(),
    ];
    for variable in variable_by_job.values() {
        lines.push(format!("  --result \"${variable}\" \\"));
    }
    lines.push("  >target/acceptance-impact/shadow-report.json".to_owned());
    lines.push(
        "jq '{status, selector_outcome, selected_ci_jobs, full_failures, observed_misses}' \\"
            .to_owned(),
    );
    lines.push(
        "  target/acceptance-impact/shadow-report.json >>\"$GITHUB_STEP_SUMMARY\"".to_owned(),
    );
    lines.join("\n")
}

fn validate_shadow_upload_step(value: &Yaml, job_path: &str) -> Result<()> {
    let step_path = format!("{job_path}.steps[6]");
    let step = as_mapping(value, &step_path)?;
    require_exact_step_string(step, "if", FAN_IN_IF, job_path, 6)?;
    require_exact_step_string(step, "uses", "actions/upload-artifact@v7", job_path, 6)?;
    forbid_step_modifiers(step, job_path, 6, false)?;
    let with = as_mapping(
        required(step, "with", &step_path)?,
        &format!("{step_path}.with"),
    )?;
    let expected = [
        ("name", "deve-impact-shadow-${{ github.sha }}"),
        ("path", "target/acceptance-impact/*.json"),
        ("if-no-files-found", "error"),
        ("retention-days", "14"),
    ];
    if with.len() != expected.len() {
        bail!(
            "acceptance producers: {step_path}.with must contain the exact diagnostic upload contract"
        );
    }
    for (key, value) in expected {
        if scalar_text(required(with, key, &format!("{step_path}.with"))?)? != value {
            bail!("acceptance producers: {step_path}.with.{key} is not exact");
        }
    }
    Ok(())
}

fn require_exact_step_string(
    step: &Hash,
    key: &str,
    expected: &str,
    job_path: &str,
    index: usize,
) -> Result<()> {
    let path = format!("{job_path}.steps[{index}].{key}");
    if as_string(
        required(step, key, &format!("{job_path}.steps[{index}]"))?,
        &path,
    )?
    .trim()
        != expected
    {
        bail!("acceptance producers: {path} is not exact");
    }
    Ok(())
}

fn forbid_step_modifiers(step: &Hash, job_path: &str, index: usize, allow_env: bool) -> Result<()> {
    for key in [
        "continue-on-error",
        "shell",
        "timeout-minutes",
        "working-directory",
    ] {
        if optional(step, key).is_some() {
            bail!("acceptance producers: {job_path}.steps[{index}] may not declare {key}");
        }
    }
    if !allow_env && optional(step, "env").is_some() {
        bail!("acceptance producers: {job_path}.steps[{index}] may not declare env");
    }
    Ok(())
}
