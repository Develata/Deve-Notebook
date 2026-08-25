//! Producer registry parsing and contract validation.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

mod ci_workflow;
mod producer_validation;
mod release_workflow;

pub(super) use self::producer_validation::contract_fingerprint;
use self::producer_validation::validate_producer;
#[cfg(test)]
use self::producer_validation::{
    artifact_location_allowed, sensitive_env_name, valid_env_name, valid_identifier,
    validate_shell_invocation, validate_step,
};
use super::model::{PRODUCER_REGISTRY_PATH, Producer, ProducerRegistry};
use crate::acceptance_matrix::model::MatrixRow;
use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub(super) fn read_and_validate(root: &Path, rows: &[MatrixRow]) -> Result<ProducerRegistry> {
    let path = root.join(PRODUCER_REGISTRY_PATH);
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let registry: ProducerRegistry = serde_json::from_str(&content)
        .with_context(|| format!("invalid producer registry {}", path.display()))?;
    validate(root, rows, &registry)?;
    Ok(registry)
}

fn validate(root: &Path, rows: &[MatrixRow], registry: &ProducerRegistry) -> Result<()> {
    if registry.schema != 3 {
        bail!(
            "acceptance producers: unsupported schema {}",
            registry.schema
        );
    }
    let matrix_evidence = matrix_executable_evidence(rows)?;
    let required: BTreeSet<_> = rows
        .iter()
        .filter(|row| {
            row.requirement == "required"
                && ((row.gate == "tag-ready" && row.evidence_kind == "receipt")
                    || (row.gate == "ci"
                        && matches!(row.evidence_kind.as_str(), "test" | "script")))
        })
        .map(|row| row.evidence_id.as_str())
        .collect();
    let mut producer_ids = BTreeSet::new();
    let mut owners = BTreeMap::<&str, &str>::new();
    for producer in &registry.producers {
        if !producer_ids.insert(producer.producer_id.as_str()) {
            bail!(
                "acceptance producers: duplicate producer_id {}",
                producer.producer_id
            );
        }
        for evidence_id in &producer.evidence_ids {
            if !matrix_evidence.contains_key(evidence_id.as_str()) {
                bail!(
                    "acceptance producers: {} references unknown or non-executable evidence {}",
                    producer.producer_id,
                    evidence_id
                );
            }
            if let Some(previous) = owners.insert(evidence_id, &producer.producer_id) {
                bail!(
                    "acceptance producers: evidence {evidence_id} is owned by both {previous} and {}",
                    producer.producer_id
                );
            }
        }
        validate_producer(root, producer, &matrix_evidence)?;
        for evidence_id in producer.claims_env.keys() {
            if !producer.evidence_ids.contains(evidence_id) {
                bail!(
                    "acceptance producers: claims binding {evidence_id} is not produced by {}",
                    producer.producer_id
                );
            }
        }
    }
    validate_dependencies(registry, &producer_ids)?;
    ci_workflow::validate(root, registry)?;
    release_workflow::validate(root, rows, registry)?;
    let missing: Vec<_> = required
        .into_iter()
        .filter(|evidence_id| !owners.contains_key(evidence_id))
        .collect();
    if !missing.is_empty() {
        bail!(
            "acceptance producers: required executable evidence has no producer: {}",
            missing.join(", ")
        );
    }
    Ok(())
}

fn matrix_executable_evidence(rows: &[MatrixRow]) -> Result<BTreeMap<&str, &MatrixRow>> {
    let mut result = BTreeMap::new();
    for row in rows
        .iter()
        .filter(|row| matches!(row.evidence_kind.as_str(), "test" | "script" | "receipt"))
    {
        if let Some(previous) = result.insert(row.evidence_id.as_str(), row) {
            for field in ["evidence_kind", "evidence_ref", "surface", "mode"] {
                let equal = match field {
                    "evidence_kind" => previous.evidence_kind == row.evidence_kind,
                    "evidence_ref" => previous.evidence_ref == row.evidence_ref,
                    "surface" => previous.surface == row.surface,
                    "mode" => previous.mode == row.mode,
                    _ => unreachable!(),
                };
                if !equal {
                    bail!(
                        "acceptance producers: repeated evidence {} has inconsistent {field}",
                        row.evidence_id
                    );
                }
            }
        }
    }
    Ok(result)
}

fn validate_dependencies(registry: &ProducerRegistry, producer_ids: &BTreeSet<&str>) -> Result<()> {
    let producers = registry
        .producers
        .iter()
        .map(|producer| (producer.producer_id.as_str(), producer))
        .collect::<BTreeMap<_, _>>();
    for producer in &registry.producers {
        for dependency in &producer.dependencies {
            if dependency == &producer.producer_id {
                bail!(
                    "acceptance producers: {} may not depend on itself",
                    producer.producer_id
                );
            }
            if !producer_ids.contains(dependency.as_str()) {
                bail!(
                    "acceptance producers: {} references unknown dependency {dependency}",
                    producer.producer_id
                );
            }
            let dependency_producer = producers[dependency.as_str()];
            if producer
                .tiers
                .iter()
                .any(|tier| !dependency_producer.tiers.contains(tier))
            {
                bail!(
                    "acceptance producers: dependency {dependency} must support every tier used by {}",
                    producer.producer_id
                );
            }
        }
    }

    fn visit<'a>(
        id: &'a str,
        producers: &BTreeMap<&'a str, &'a Producer>,
        visiting: &mut BTreeSet<&'a str>,
        visited: &mut BTreeSet<&'a str>,
    ) -> Result<()> {
        if visited.contains(id) {
            return Ok(());
        }
        if !visiting.insert(id) {
            bail!("acceptance producers: dependency cycle includes {id}");
        }
        for dependency in &producers[id].dependencies {
            visit(dependency, producers, visiting, visited)?;
        }
        visiting.remove(id);
        visited.insert(id);
        Ok(())
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for id in producers.keys().copied() {
        visit(id, &producers, &mut visiting, &mut visited)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
