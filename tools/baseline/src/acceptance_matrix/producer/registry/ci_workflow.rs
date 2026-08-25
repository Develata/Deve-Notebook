//! Fail-closed projection from CI producer authority to compatible workflow shards.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::super::model::{Producer, ProducerRegistry};
use super::super::runner::FINALLY_STEP_TIMEOUT_SECONDS;
use anyhow::{Context, Result, bail};
use command::{
    job_commands, reject_tolerated_or_conditional, require_node_24, require_rust_toolchain,
};
use fan_in::validate_fan_in;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use yaml::{as_mapping, as_sequence, as_string, as_u64, optional, required};
use yaml_rust2::YamlLoader;

mod command;
mod fan_in;
#[cfg(test)]
mod tests;
mod yaml;

const BUILD_MARGIN_MINUTES: u64 = 15;

pub(super) fn validate(root: &Path, registry: &ProducerRegistry) -> Result<()> {
    let path = root.join(".github/workflows/check.yml");
    let workflow = fs::read_to_string(&path)
        .with_context(|| format!("acceptance producers: failed to read {}", path.display()))?;
    validate_text(&workflow, registry)
}

fn validate_text(workflow: &str, registry: &ProducerRegistry) -> Result<()> {
    let documents = YamlLoader::load_from_str(workflow)
        .context("acceptance producers: check.yml is not valid YAML")?;
    if documents.len() != 1 {
        bail!("acceptance producers: check.yml must contain exactly one YAML document");
    }
    let root = as_mapping(&documents[0], "check.yml")?;
    for key in ["defaults", "env"] {
        if optional(root, key).is_some() {
            bail!(
                "acceptance producers: check.yml may not declare workflow {key}; canonical producer commands must retain their execution semantics"
            );
        }
    }
    let jobs = as_mapping(required(root, "jobs", "check.yml")?, "check.yml.jobs")?;
    let ci_producers = registry
        .producers
        .iter()
        .filter(|producer| producer.tiers.iter().any(|tier| tier == "ci"))
        .map(|producer| (producer.producer_id.as_str(), producer))
        .collect::<BTreeMap<_, _>>();
    let mut counts = ci_producers
        .keys()
        .map(|id| (*id, 0usize))
        .collect::<BTreeMap<_, _>>();
    let mut execution_jobs = BTreeSet::new();

    for (job_key, value) in jobs {
        let job_id = as_string(job_key, "check.yml.jobs key")?;
        let job_path = format!("check.yml.jobs.{job_id}");
        let job = as_mapping(value, &job_path)?;
        let commands = job_commands(job, &job_path)?;
        if commands.executions.is_empty() {
            continue;
        }
        reject_tolerated_or_conditional(job, &job_path)?;
        command::reject_job_execution_modifiers(job, &job_path)?;
        if commands.executions.len() != 1 || commands.plans.len() != 1 {
            bail!(
                "acceptance producers: {job_path} must contain exactly one filtered CI plan and one execution command"
            );
        }
        let selected = &commands.executions[0];
        if &commands.plans[0] != selected {
            bail!("acceptance producers: {job_path} CI plan and execution producer sets differ");
        }
        let host = canonical_host(as_string(
            required(job, "runs-on", &job_path)?,
            &format!("{job_path}.runs-on"),
        )?)?;
        let timeout_minutes = as_u64(
            required(job, "timeout-minutes", &job_path)?,
            &format!("{job_path}.timeout-minutes"),
        )?;
        let steps = as_sequence(
            required(job, "steps", &job_path)?,
            &format!("{job_path}.steps"),
        )?;
        let first_command_step = commands
            .first_command_step
            .expect("execution job contains a CI command");
        require_rust_toolchain(steps, first_command_step, &job_path)?;

        let mut required_timeout_seconds = 0u64;
        let mut needs_node = false;
        for producer_id in selected {
            let producer = ci_producers.get(producer_id.as_str()).with_context(|| {
                format!(
                    "acceptance producers: {job_path} executes unknown CI producer {producer_id}"
                )
            })?;
            validate_selected_producer(producer, selected, host, &job_path)?;
            required_timeout_seconds = required_timeout_seconds
                .checked_add(producer.timeout_seconds)
                .with_context(|| {
                    format!("acceptance producers: {job_path} cumulative producer timeout overflow")
                })?;
            let cleanup_timeout_seconds = (producer.finally_steps.len() as u64)
                .checked_mul(FINALLY_STEP_TIMEOUT_SECONDS)
                .context("acceptance producers: producer cleanup timeout overflow")?;
            required_timeout_seconds = required_timeout_seconds
                .checked_add(cleanup_timeout_seconds)
                .context("acceptance producers: cumulative cleanup timeout overflow")?;
            needs_node |= producer.steps.iter().any(|step| step.program == "node");
            *counts
                .get_mut(producer.producer_id.as_str())
                .expect("CI producer count initialized") += 1;
        }
        let required_minutes = required_timeout_seconds
            .div_ceil(60)
            .saturating_add(BUILD_MARGIN_MINUTES);
        if timeout_minutes < required_minutes {
            bail!(
                "acceptance producers: {job_path} timeout {timeout_minutes}m is below producer deadline plus build margin {required_minutes}m"
            );
        }
        if needs_node {
            require_node_24(steps, first_command_step, &job_path)?;
        }
        execution_jobs.insert(job_id.to_owned());
    }

    validate_exactly_once(&counts)?;
    validate_fan_in(jobs, &execution_jobs)?;
    Ok(())
}

fn validate_selected_producer(
    producer: &Producer,
    selected: &BTreeSet<String>,
    host: &str,
    job_path: &str,
) -> Result<()> {
    if !producer.host_os.iter().any(|allowed| allowed == host) {
        bail!(
            "acceptance producers: {job_path} runs {} on incompatible host {host}",
            producer.producer_id
        );
    }
    for dependency in &producer.dependencies {
        if !selected.contains(dependency) {
            bail!(
                "acceptance producers: {job_path} splits dependency {dependency} from {}",
                producer.producer_id
            );
        }
    }
    Ok(())
}

fn validate_exactly_once(counts: &BTreeMap<&str, usize>) -> Result<()> {
    let missing = counts
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    let duplicate = counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(id, count)| format!("{id} ({count})"))
        .collect::<Vec<_>>();
    if !missing.is_empty() || !duplicate.is_empty() {
        bail!(
            "acceptance producers: check.yml must execute every CI producer exactly once; missing=[{}] duplicate=[{}]",
            missing.join(", "),
            duplicate.join(", ")
        );
    }
    Ok(())
}

fn canonical_host(runs_on: &str) -> Result<&'static str> {
    match runs_on {
        "ubuntu-latest" => Ok("linux"),
        "windows-latest" => Ok("windows"),
        "macos-latest" => Ok("macos"),
        other => bail!(
            "acceptance producers: CI producer shard requires a fixed supported runs-on host, found {other}"
        ),
    }
}
