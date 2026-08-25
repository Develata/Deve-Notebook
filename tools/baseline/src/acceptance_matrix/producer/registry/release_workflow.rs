//! Fail-closed candidate workflow projection for receipt producers.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::super::model::ProducerRegistry;
use crate::acceptance_matrix::model::MatrixRow;
use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use yaml_rust2::{Yaml, YamlLoader};

const WORKFLOWS: [&str; 2] = [
    ".github/workflows/release-candidate.yml",
    ".github/workflows/release-native.yml",
];
mod command;
mod job;
mod receipt_artifacts;
mod structure;

use command::parse_candidate_command;

pub(super) fn validate(root: &Path, rows: &[MatrixRow], registry: &ProducerRegistry) -> Result<()> {
    validate_tag_ready_bindings(rows, registry)?;
    let producer_timeouts = registry
        .producers
        .iter()
        .map(|producer| (producer.producer_id.as_str(), producer.timeout_seconds))
        .collect::<BTreeMap<_, _>>();
    let contents = WORKFLOWS
        .iter()
        .map(|relative| {
            let path = root.join(relative);
            fs::read_to_string(&path)
                .with_context(|| format!("acceptance producers: failed to read {}", path.display()))
        })
        .collect::<Result<Vec<_>>>()?;
    structure::validate_candidate(&contents[0])?;
    receipt_artifacts::validate(&contents[0], &contents[1])?;

    let mut actual = BTreeMap::<String, usize>::new();
    for (relative, content) in WORKFLOWS.into_iter().zip(&contents) {
        collect_workflow(relative, content, &mut actual, &producer_timeouts)?;
    }
    validate_counts(registry, &actual)
}

fn validate_tag_ready_bindings(rows: &[MatrixRow], registry: &ProducerRegistry) -> Result<()> {
    let owners = registry
        .producers
        .iter()
        .flat_map(|producer| {
            producer
                .evidence_ids
                .iter()
                .map(move |evidence| (evidence.as_str(), producer))
        })
        .collect::<BTreeMap<_, _>>();
    for row in rows.iter().filter(|row| {
        row.gate == "tag-ready" && row.requirement == "required" && row.evidence_kind == "receipt"
    }) {
        let producer = owners.get(row.evidence_id.as_str()).with_context(|| {
            format!(
                "acceptance producers: tag-ready receipt {} has no producer",
                row.evidence_id
            )
        })?;
        if !producer.candidate_required {
            bail!(
                "acceptance producers: tag-ready receipt {} owner {} must be candidate_required",
                row.evidence_id,
                producer.producer_id
            );
        }
    }
    Ok(())
}

fn collect_workflow(
    label: &str,
    content: &str,
    counts: &mut BTreeMap<String, usize>,
    producer_timeouts: &BTreeMap<&str, u64>,
) -> Result<()> {
    let documents = YamlLoader::load_from_str(content)
        .with_context(|| format!("acceptance producers: {label} is not valid YAML"))?;
    let [document] = documents.as_slice() else {
        bail!("acceptance producers: {label} must contain one YAML document");
    };
    let root = mapping(document, label)?;
    if optional(root, "defaults").is_some() {
        bail!("acceptance producers: {label} may not override workflow run defaults");
    }
    let jobs = mapping(required(root, "jobs", label)?, &format!("{label}.jobs"))?;
    for (job_key, job_value) in jobs {
        let job_id = string(job_key, &format!("{label}.jobs key"))?;
        let job_path = format!("{label}.jobs.{job_id}");
        let job = mapping(job_value, &job_path)?;
        let Some(steps) = optional(job, "steps") else {
            continue;
        };
        let steps = sequence(steps, &format!("{job_path}.steps"))?;
        let mut job_producers = Vec::new();
        for (index, step_value) in steps.iter().enumerate() {
            let step_path = format!("{job_path}.steps[{index}]");
            let step = mapping(step_value, &step_path)?;
            let Some(run) = optional(step, "run") else {
                continue;
            };
            let run = string(run, &format!("{step_path}.run"))?;
            let shell = optional(step, "shell")
                .map(|value| string(value, &format!("{step_path}.shell")))
                .transpose()?;
            let producers = parse_candidate_command(run, shell, &step_path)?;
            let Some(producers) = producers else {
                continue;
            };
            for key in ["if", "continue-on-error"] {
                if optional(step, key).is_some() || optional(job, key).is_some() {
                    bail!(
                        "acceptance producers: {step_path} candidate producer may not be conditional or tolerated"
                    );
                }
            }
            if optional(job, "strategy").is_some() {
                bail!(
                    "acceptance producers: {job_path} candidate producer may not use a matrix strategy"
                );
            }
            job::validate_defaults(job, &job_path)?;
            if optional(step, "working-directory").is_some()
                || optional(step, "timeout-minutes").is_some()
            {
                bail!(
                    "acceptance producers: {step_path} candidate producer may not reinterpret execution defaults"
                );
            }
            for producer in producers {
                *counts.entry(producer.clone()).or_default() += 1;
                job_producers.push(producer);
            }
        }
        if !job_producers.is_empty() {
            job::validate_budget(&job_path, job, &job_producers, producer_timeouts)?;
        }
    }
    Ok(())
}

fn validate_counts(registry: &ProducerRegistry, actual: &BTreeMap<String, usize>) -> Result<()> {
    let known = registry
        .producers
        .iter()
        .map(|producer| producer.producer_id.as_str())
        .collect::<BTreeSet<_>>();
    for (producer, count) in actual {
        if !known.contains(producer.as_str()) {
            bail!("acceptance producers: candidate workflows invoke unknown producer {producer}");
        }
        if *count != 1 {
            bail!(
                "acceptance producers: candidate producer {producer} must be invoked exactly once, found {count}"
            );
        }
    }
    let expected = registry
        .producers
        .iter()
        .filter(|producer| producer.candidate_required)
        .map(|producer| producer.producer_id.as_str())
        .collect::<BTreeSet<_>>();
    let observed = actual.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let missing = expected.difference(&observed).copied().collect::<Vec<_>>();
    let extra = observed.difference(&expected).copied().collect::<Vec<_>>();
    if !missing.is_empty() || !extra.is_empty() {
        bail!(
            "acceptance producers: candidate workflow projection mismatch; missing=[{}] extra=[{}]",
            missing.join(","),
            extra.join(",")
        );
    }
    Ok(())
}

fn mapping<'a>(value: &'a Yaml, path: &str) -> Result<&'a yaml_rust2::yaml::Hash> {
    value
        .as_hash()
        .with_context(|| format!("acceptance producers: {path} must be a mapping"))
}

fn sequence<'a>(value: &'a Yaml, path: &str) -> Result<&'a [Yaml]> {
    value
        .as_vec()
        .map(Vec::as_slice)
        .with_context(|| format!("acceptance producers: {path} must be a sequence"))
}

fn string<'a>(value: &'a Yaml, path: &str) -> Result<&'a str> {
    value
        .as_str()
        .with_context(|| format!("acceptance producers: {path} must be a string"))
}

fn optional<'a>(mapping: &'a yaml_rust2::yaml::Hash, key: &str) -> Option<&'a Yaml> {
    mapping.get(&Yaml::String(key.to_owned()))
}

fn required<'a>(mapping: &'a yaml_rust2::yaml::Hash, key: &str, path: &str) -> Result<&'a Yaml> {
    optional(mapping, key).with_context(|| format!("acceptance producers: {path} missing {key}"))
}

#[cfg(test)]
mod tests;
