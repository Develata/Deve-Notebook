//! Stable required-check fan-in validation.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::command::{
    reject_job_execution_modifiers, reject_step_execution_modifiers,
    reject_tolerated_or_conditional,
};
use super::yaml::{as_mapping, as_sequence, as_string, optional, required};
use anyhow::{Result, bail};
use std::collections::BTreeSet;
use yaml_rust2::yaml::Hash;

const FAN_IN_JOB: &str = "check";
const FAN_IN_IF: &str = "${{ always() }}";

pub(super) fn validate_fan_in(jobs: &Hash, execution_jobs: &BTreeSet<String>) -> Result<()> {
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
    let mut ordered_needs = vec!["core-checks".to_owned()];
    ordered_needs.extend(execution_jobs.iter().cloned());
    ordered_needs.push("watcher-native-fs".to_owned());
    let expected_needs = ordered_needs.iter().cloned().collect::<BTreeSet<_>>();
    let needs = as_sequence(required(job, "needs", &path)?, &format!("{path}.needs"))?
        .iter()
        .map(|value| as_string(value, &format!("{path}.needs")))
        .collect::<Result<BTreeSet<_>>>()?
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if needs != expected_needs {
        bail!(
            "acceptance producers: {path} needs mismatch; expected={expected_needs:?} actual={needs:?}"
        );
    }
    for dependency in &needs {
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
