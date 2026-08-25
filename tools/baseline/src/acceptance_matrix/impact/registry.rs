//! Fail-closed validation for the shadow-only CI impact registry.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::model::{IMPACT_REGISTRY_PATH, ImpactModule, ImpactRegistry};
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

mod path_rules;
mod shards;

pub(super) struct LoadedRegistry {
    pub(super) registry: ImpactRegistry,
    pub(super) impact_fingerprint: String,
}

pub(super) fn load(root: &Path, producer_ids: &BTreeSet<String>) -> Result<LoadedRegistry> {
    let path = root.join(IMPACT_REGISTRY_PATH);
    let content = fs::read(&path)
        .with_context(|| format!("acceptance-impact: failed to read {}", path.display()))?;
    let registry: ImpactRegistry = serde_json::from_slice(&content)
        .with_context(|| format!("acceptance-impact: invalid registry {}", path.display()))?;
    let tracked = path_rules::tracked_paths(root)?;
    validate(&registry, producer_ids, &tracked)?;
    Ok(LoadedRegistry {
        registry,
        impact_fingerprint: format!("sha256:{:x}", Sha256::digest(&content)),
    })
}

fn validate(
    registry: &ImpactRegistry,
    producer_ids: &BTreeSet<String>,
    tracked: &[String],
) -> Result<()> {
    if registry.schema != 1 || registry.mode != "shadow-only" {
        bail!("acceptance-impact: registry must use schema 1 and shadow-only mode");
    }
    let artifacts = unique_ids(
        registry
            .artifacts
            .iter()
            .map(|artifact| artifact.artifact_id.as_str()),
        "artifact",
    )?;
    let artifact_contracts = BTreeMap::from([
        ("android-arm64-apk", "apk-digest-and-signer"),
        ("android-x86-apk", "apk-digest"),
        ("docker-image", "image-id-and-archive-digest"),
        ("macos-package", "candidate-file-digest"),
        ("receipt-bundle", "producer-contract-and-head"),
        ("remote-fixture", "script-and-config-digest"),
        ("source", "exact-head"),
        ("web-dist", "content-digest"),
        ("windows-package", "candidate-file-digest"),
    ]);
    if artifacts != artifact_contracts.keys().copied().collect()
        || registry.artifacts.iter().any(|artifact| {
            artifact_contracts.get(artifact.artifact_id.as_str())
                != Some(&artifact.identity.as_str())
        })
    {
        bail!("acceptance-impact: artifact identity contracts are incomplete or unknown");
    }
    let modules = unique_ids(
        registry
            .modules
            .iter()
            .map(|module| module.module_id.as_str()),
        "module",
    )?;
    let shards = unique_ids(
        registry.shards.iter().map(|shard| shard.shard_id.as_str()),
        "shard",
    )?;
    validate_profiles(registry, &shards)?;
    path_rules::validate_modules(registry, &modules, &shards, tracked)?;
    shards::validate(registry, &artifacts, producer_ids)?;
    validate_module_cycles(&registry.modules, &modules)
}

fn validate_profiles(registry: &ImpactRegistry, shard_ids: &BTreeSet<&str>) -> Result<()> {
    let profiles = unique_ids(
        registry
            .profiles
            .iter()
            .map(|profile| profile.profile_id.as_str()),
        "profile",
    )?;
    let expected = BTreeMap::from([
        ("candidate-full-release", "full"),
        ("diagnostic-module", "selective"),
        ("main-full-source", "full"),
        ("nightly-full-system", "full"),
        ("pr-selective", "selective"),
    ]);
    if profiles != expected.keys().copied().collect() {
        bail!("acceptance-impact: fixed profile set is incomplete or contains an unknown profile");
    }
    let source_shards = registry
        .shards
        .iter()
        .filter(|shard| shard.layer == "source")
        .map(|shard| shard.shard_id.as_str())
        .collect::<BTreeSet<_>>();
    let runtime_shards = registry
        .shards
        .iter()
        .filter(|shard| shard.layer == "runtime")
        .map(|shard| shard.shard_id.as_str())
        .collect::<BTreeSet<_>>();
    if source_shards.is_empty()
        || runtime_shards.is_empty()
        || registry
            .shards
            .iter()
            .any(|shard| !matches!(shard.layer.as_str(), "source" | "runtime"))
    {
        bail!(
            "acceptance-impact: registry requires nonempty controlled source and runtime shard layers"
        );
    }
    for profile in &registry.profiles {
        if expected[profile.profile_id.as_str()] != profile.selection {
            bail!(
                "acceptance-impact: profile {} must use {} selection",
                profile.profile_id,
                expected[profile.profile_id.as_str()]
            );
        }
        let selected = unique_values(&profile.shards, "profile shard")?;
        if selected.is_empty() || !selected.is_subset(shard_ids) {
            bail!(
                "acceptance-impact: profile {} has an empty or unknown shard set",
                profile.profile_id
            );
        }
        let always = unique_values(&profile.always_shards, "always shard")?;
        if !always.is_subset(&selected) {
            bail!(
                "acceptance-impact: profile {} always shard must be eligible in that profile",
                profile.profile_id
            );
        }
        if profile.selection == "selective" && !selected.is_subset(&source_shards) {
            bail!(
                "acceptance-impact: selective profile {} may only select source shards",
                profile.profile_id
            );
        }
        if profile.profile_id == "main-full-source" && selected != source_shards {
            bail!("acceptance-impact: main-full-source must enumerate every source shard");
        }
        if matches!(
            profile.profile_id.as_str(),
            "nightly-full-system" | "candidate-full-release"
        ) && &selected != shard_ids
        {
            bail!(
                "acceptance-impact: profile {} must enumerate every system shard",
                profile.profile_id
            );
        }
    }
    Ok(())
}

fn validate_module_cycles(modules: &[ImpactModule], ids: &BTreeSet<&str>) -> Result<()> {
    let graph = modules
        .iter()
        .map(|module| (module.module_id.as_str(), module))
        .collect::<BTreeMap<_, _>>();
    fn visit<'a>(
        id: &'a str,
        graph: &BTreeMap<&'a str, &'a ImpactModule>,
        visiting: &mut BTreeSet<&'a str>,
        visited: &mut BTreeSet<&'a str>,
    ) -> Result<()> {
        if visited.contains(id) {
            return Ok(());
        }
        if !visiting.insert(id) {
            bail!("acceptance-impact: module dependency cycle includes {id}");
        }
        for dependency in &graph[id].dependencies {
            visit(dependency, graph, visiting, visited)?;
        }
        visiting.remove(id);
        visited.insert(id);
        Ok(())
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for id in ids {
        visit(id, &graph, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn unique_ids<'a>(values: impl Iterator<Item = &'a str>, label: &str) -> Result<BTreeSet<&'a str>> {
    let values = values.collect::<Vec<_>>();
    let unique = values.iter().copied().collect::<BTreeSet<_>>();
    if values.len() != unique.len() || unique.iter().any(|value| !valid_id(value)) {
        bail!("acceptance-impact: {label} identifiers must be unique and canonical");
    }
    Ok(unique)
}

fn unique_values<'a>(values: &'a [String], label: &str) -> Result<BTreeSet<&'a str>> {
    unique_ids(values.iter().map(String::as_str), label)
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '-'))
}
