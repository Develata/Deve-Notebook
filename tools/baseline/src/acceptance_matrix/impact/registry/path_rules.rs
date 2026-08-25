//! Repository path ownership validation for dependency-aware acceptance planning.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::super::model::ImpactRegistry;
use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};
use std::process::Command;

pub(super) fn validate_modules(
    registry: &ImpactRegistry,
    module_ids: &BTreeSet<&str>,
    shard_ids: &BTreeSet<&str>,
    tracked: &[String],
) -> Result<()> {
    let required_module_ids = [
        "cli-runtime",
        "core-authority",
        "delivery-config",
        "desktop-adapter",
        "governance-contracts",
        "integration-harness",
        "mobile-adapter",
        "plugin-runtime",
        "web-shell",
        "workspace-toolchain",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if module_ids != &required_module_ids {
        bail!("acceptance-impact: fixed capability module set is incomplete or unknown");
    }
    let mut exact = BTreeMap::<&str, &str>::new();
    let mut prefixes = Vec::<(&str, &str)>::new();
    for module in &registry.modules {
        if module.exact_paths.is_empty() && module.path_prefixes.is_empty() {
            bail!(
                "acceptance-impact: module {} has no path rules",
                module.module_id
            );
        }
        let dependencies = super::unique_values(&module.dependencies, "module dependency")?;
        if dependencies.contains(module.module_id.as_str()) || !dependencies.is_subset(module_ids) {
            bail!(
                "acceptance-impact: module {} has an unknown or self dependency",
                module.module_id
            );
        }
        let module_shards = super::unique_values(&module.shards, "module shard")?;
        if module_shards.is_empty() || !module_shards.is_subset(shard_ids) {
            bail!(
                "acceptance-impact: module {} has an invalid shard set",
                module.module_id
            );
        }
        validate_required_topology(
            &module.module_id,
            module.full_trigger,
            &dependencies,
            &module_shards,
        )?;
        for path in &module.exact_paths {
            validate_repo_path(path, false)?;
            if let Some(owner) = exact.insert(path, &module.module_id) {
                bail!(
                    "acceptance-impact: exact path {path} is owned by {owner} and {}",
                    module.module_id
                );
            }
        }
        for prefix in &module.path_prefixes {
            validate_repo_path(prefix, true)?;
            prefixes.push((prefix, &module.module_id));
        }
    }
    for left in 0..prefixes.len() {
        for right in left + 1..prefixes.len() {
            if prefixes[left].0.starts_with(prefixes[right].0)
                || prefixes[right].0.starts_with(prefixes[left].0)
            {
                bail!(
                    "acceptance-impact: overlapping prefixes {} and {} are ambiguous",
                    prefixes[left].0,
                    prefixes[right].0
                );
            }
        }
    }
    for path in tracked {
        if exact.contains_key(path.as_str()) {
            continue;
        }
        let owners = prefixes
            .iter()
            .filter(|(prefix, _)| path.starts_with(prefix))
            .map(|(_, owner)| *owner)
            .collect::<BTreeSet<_>>();
        if owners.len() != 1 {
            bail!(
                "acceptance-impact: tracked path {path} must map to exactly one module, found {}",
                owners.len()
            );
        }
    }
    Ok(())
}

fn validate_required_topology(
    module_id: &str,
    full_trigger: bool,
    dependencies: &BTreeSet<&str>,
    shards: &BTreeSet<&str>,
) -> Result<()> {
    let (required_full, required_dependencies, required_shards): (bool, &[&str], &[&str]) =
        match module_id {
            "governance-contracts" => (true, &[], &["contract-static", "release-supply"]),
            "workspace-toolchain" => (true, &[], &["workspace-build"]),
            "delivery-config" => (
                true,
                &["cli-runtime", "web-shell"],
                &["docker-runtime", "release-supply", "web-ci"],
            ),
            "core-authority" => (false, &[], &["core-ci", "diff-ci", "protocol-ci"]),
            "cli-runtime" => (
                false,
                &["core-authority"],
                &[
                    "diff-ci",
                    "protocol-ci",
                    "repo-process",
                    "source-tag-contracts",
                ],
            ),
            "web-shell" => (
                false,
                &["core-authority"],
                &["diff-ci", "protocol-ci", "source-tag-contracts", "web-ci"],
            ),
            "desktop-adapter" => (
                false,
                &["cli-runtime", "core-authority", "web-shell"],
                &["desktop-runtime", "native-ci"],
            ),
            "mobile-adapter" => (
                false,
                &["cli-runtime", "core-authority", "web-shell"],
                &["mobile-runtime", "native-ci", "protocol-ci"],
            ),
            "plugin-runtime" => (false, &["cli-runtime", "core-authority"], &["ai-plugin-ci"]),
            "integration-harness" => (false, &["plugin-runtime"], &["ai-plugin-ci"]),
            _ => bail!("acceptance-impact: unknown capability module {module_id}"),
        };
    if full_trigger != required_full
        || dependencies
            != &required_dependencies
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
        || shards != &required_shards.iter().copied().collect::<BTreeSet<_>>()
    {
        bail!(
            "acceptance-impact: module {module_id} is missing a required dependency edge or shard"
        );
    }
    Ok(())
}

pub(super) fn tracked_paths(root: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .context("acceptance-impact: failed to enumerate tracked paths")?;
    if !output.status.success() {
        bail!("acceptance-impact: git ls-files failed");
    }
    output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| {
            std::str::from_utf8(path)
                .map(str::to_owned)
                .context("acceptance-impact: tracked path is not UTF-8")
        })
        .collect()
}

fn validate_repo_path(value: &str, prefix: bool) -> Result<()> {
    if value.is_empty()
        || value.contains('\\')
        || prefix != value.ends_with('/')
        || Path::new(value)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("acceptance-impact: invalid repository path rule {value:?}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_repo_path, validate_required_topology};
    use std::collections::BTreeSet;

    #[test]
    fn path_rules_are_forward_slash_and_kind_exact() {
        assert!(validate_repo_path("apps/web/", true).is_ok());
        assert!(validate_repo_path("Cargo.toml", false).is_ok());
        assert!(validate_repo_path("apps\\web\\", true).is_err());
        assert!(validate_repo_path("apps/web/../cli/", true).is_err());
        assert!(validate_repo_path("apps/web", true).is_err());
    }

    #[test]
    fn required_topology_rejects_deleted_consumer_edge_or_shard() {
        let dependencies = BTreeSet::from(["core-authority"]);
        let shards = BTreeSet::from(["diff-ci", "protocol-ci", "source-tag-contracts", "web-ci"]);
        validate_required_topology("web-shell", false, &dependencies, &shards).unwrap();
        assert!(validate_required_topology("web-shell", false, &BTreeSet::new(), &shards).is_err());
        assert!(
            validate_required_topology(
                "web-shell",
                false,
                &dependencies,
                &BTreeSet::from(["diff-ci", "protocol-ci", "web-ci"])
            )
            .is_err()
        );
        assert!(validate_required_topology("web-shell", true, &dependencies, &shards).is_err());
    }
}
