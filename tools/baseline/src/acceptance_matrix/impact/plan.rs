//! Deterministic shadow-only module and shard planning.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::model::{ImpactModule, ImpactPlan, ImpactRegistry, InputFingerprints};
use crate::acceptance_matrix::model::MatrixRow;
use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

pub(in crate::acceptance_matrix::impact) mod input;
pub(super) use input::PlanArgs;

pub(super) fn build(
    root: &Path,
    registry: &ImpactRegistry,
    input_fingerprints: InputFingerprints,
    mut args: PlanArgs,
    evidence_by_producer: &BTreeMap<String, Vec<String>>,
    rows: &[MatrixRow],
) -> Result<ImpactPlan> {
    let profile = registry
        .profiles
        .iter()
        .find(|profile| profile.profile_id == args.profile)
        .with_context(|| format!("acceptance-impact: unknown profile {}", args.profile))?;
    if let (Some(base), Some(head)) = (&args.base, &args.head) {
        args.changed_files = input::git_changed_files(root, base, head)?;
    }
    args.changed_files.sort();
    args.changed_files.dedup();
    if profile.selection == "selective" && args.changed_files.is_empty() {
        bail!("acceptance-impact: selective profiles require a nonempty change set");
    }

    let module_by_id = registry
        .modules
        .iter()
        .map(|module| (module.module_id.as_str(), module))
        .collect::<BTreeMap<_, _>>();
    let mut changed_modules = BTreeSet::new();
    let mut full_reasons = BTreeSet::new();
    for path in &args.changed_files {
        match classify_path(path, &registry.modules) {
            Some(module) => {
                changed_modules.insert(module.module_id.as_str());
                if module.full_trigger {
                    full_reasons.insert(format!("full-trigger:{}", module.module_id));
                }
            }
            None => {
                full_reasons.insert(format!("unknown-path:{path}"));
            }
        }
    }
    if profile.selection == "full" {
        full_reasons.insert(format!("profile-full:{}", profile.profile_id));
    }
    let full = !full_reasons.is_empty();
    let escalates_to_system = full_reasons
        .iter()
        .any(|reason| reason.starts_with("unknown-path:") || reason.starts_with("full-trigger:"));

    let selected_modules = if full {
        module_by_id.keys().copied().collect::<BTreeSet<_>>()
    } else {
        reverse_closure(&changed_modules, &registry.modules)
    };
    let reverse_consumers = reverse_closure(&changed_modules, &registry.modules)
        .difference(&changed_modules)
        .copied()
        .collect::<BTreeSet<_>>();
    let selected_shards = if full {
        if profile.selection == "selective" || escalates_to_system {
            registry
                .shards
                .iter()
                .map(|shard| shard.shard_id.as_str())
                .collect()
        } else {
            profile.shards.iter().map(String::as_str).collect()
        }
    } else {
        let mut shards = profile
            .always_shards
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        for module in selected_modules.iter().map(|id| module_by_id[id]) {
            shards.extend(
                module
                    .shards
                    .iter()
                    .map(String::as_str)
                    .filter(|shard| profile.shards.iter().any(|eligible| eligible == shard)),
            );
        }
        shards
    };

    let shard_by_id = registry
        .shards
        .iter()
        .map(|shard| (shard.shard_id.as_str(), shard))
        .collect::<BTreeMap<_, _>>();
    let mut producers = BTreeSet::new();
    let mut checks = BTreeSet::new();
    let mut artifacts = BTreeSet::new();
    let mut isolation = BTreeMap::new();
    for shard_id in &selected_shards {
        let shard = shard_by_id[shard_id];
        producers.extend(shard.producer_ids.iter().cloned());
        checks.extend(shard.checks.iter().cloned());
        artifacts.extend(shard.artifact_inputs.iter().cloned());
        isolation.insert((*shard_id).to_owned(), shard.isolation.clone());
    }
    let mut evidence = BTreeSet::new();
    for producer in &producers {
        let owned = evidence_by_producer.get(producer).with_context(|| {
            format!("acceptance-impact: selected producer {producer} has no evidence catalog")
        })?;
        evidence.extend(owned.iter().cloned());
    }
    let cases = rows
        .iter()
        .filter(|row| row.case_id != "none" && evidence.contains(&row.evidence_id))
        .map(|row| row.case_id.clone())
        .collect::<BTreeSet<_>>();

    Ok(ImpactPlan {
        schema: 1,
        status: "shadow-only",
        input_fingerprints,
        profile: profile.profile_id.clone(),
        selection: if full { "full" } else { "selective" }.to_owned(),
        scope: if selected_shards
            .iter()
            .any(|shard_id| shard_by_id[shard_id].layer == "runtime")
        {
            "system"
        } else {
            "source"
        }
        .to_owned(),
        base: args.base,
        head: args.head,
        changed_files: args.changed_files,
        full_reasons: full_reasons.into_iter().collect(),
        changed_modules: changed_modules.into_iter().map(str::to_owned).collect(),
        reverse_consumers: reverse_consumers.into_iter().map(str::to_owned).collect(),
        selected_modules: selected_modules.into_iter().map(str::to_owned).collect(),
        selected_shards: selected_shards.into_iter().map(str::to_owned).collect(),
        producer_ids: producers.into_iter().collect(),
        evidence_ids: evidence.into_iter().collect(),
        case_ids: cases.into_iter().collect(),
        checks: checks.into_iter().collect(),
        artifact_inputs: artifacts.into_iter().collect(),
        isolation,
    })
}

fn classify_path<'a>(path: &str, modules: &'a [ImpactModule]) -> Option<&'a ImpactModule> {
    if let Some(module) = modules
        .iter()
        .find(|module| module.exact_paths.iter().any(|candidate| candidate == path))
    {
        return Some(module);
    }
    modules.iter().find(|module| {
        module
            .path_prefixes
            .iter()
            .any(|prefix| path.starts_with(prefix))
    })
}

fn reverse_closure<'a>(
    changed: &BTreeSet<&'a str>,
    modules: &'a [ImpactModule],
) -> BTreeSet<&'a str> {
    let mut consumers = BTreeMap::<&str, Vec<&str>>::new();
    for module in modules {
        for dependency in &module.dependencies {
            consumers
                .entry(dependency)
                .or_default()
                .push(module.module_id.as_str());
        }
    }
    let mut selected = changed.clone();
    let mut queue = changed.iter().copied().collect::<VecDeque<_>>();
    while let Some(module) = queue.pop_front() {
        for consumer in consumers.get(module).into_iter().flatten() {
            if selected.insert(consumer) {
                queue.push_back(consumer);
            }
        }
    }
    selected
}

#[cfg(test)]
mod tests;
