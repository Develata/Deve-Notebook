//! Impact-planning regression tests.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::{PlanArgs, build, input, reverse_closure};
use crate::acceptance_matrix::impact::model::{
    ImpactModule, ImpactProfile, ImpactRegistry, ImpactShard, InputFingerprints, Isolation,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn module(id: &str, dependencies: &[&str]) -> ImpactModule {
    ImpactModule {
        module_id: id.to_owned(),
        dependencies: dependencies
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        full_trigger: false,
        exact_paths: Vec::new(),
        path_prefixes: vec![format!("{id}/")],
        shards: vec![format!("{id}-shard")],
    }
}

fn isolation() -> Isolation {
    Isolation {
        state: "per-execution".into(),
        ports: "not-applicable".into(),
        database: "per-execution".into(),
        identity: "per-execution".into(),
        fixture: "content-addressed".into(),
        logs: "per-execution".into(),
    }
}

fn planning_registry() -> ImpactRegistry {
    let mut core = module("core", &[]);
    core.shards = vec!["core-shard".into()];
    let mut web = module("web", &["core"]);
    web.shards = vec!["web-shard".into()];
    ImpactRegistry {
        schema: 1,
        mode: "shadow-only".into(),
        artifacts: Vec::new(),
        profiles: vec![
            ImpactProfile {
                profile_id: "pr-selective".into(),
                selection: "selective".into(),
                always_shards: Vec::new(),
                shards: vec!["core-shard".into(), "web-shard".into()],
            },
            ImpactProfile {
                profile_id: "candidate-full-release".into(),
                selection: "full".into(),
                always_shards: Vec::new(),
                shards: vec![
                    "core-shard".into(),
                    "web-shard".into(),
                    "runtime-shard".into(),
                ],
            },
            ImpactProfile {
                profile_id: "main-full-source".into(),
                selection: "full".into(),
                always_shards: Vec::new(),
                shards: vec!["core-shard".into(), "web-shard".into()],
            },
        ],
        modules: vec![core, web],
        shards: vec![
            ImpactShard {
                shard_id: "core-shard".into(),
                layer: "source".into(),
                execution_kind: "static".into(),
                ci_jobs: vec!["core-job".into()],
                producer_ids: vec!["ci.core".into()],
                checks: vec!["cargo.core".into()],
                artifact_inputs: vec!["source".into()],
                isolation: isolation(),
            },
            ImpactShard {
                shard_id: "web-shard".into(),
                layer: "source".into(),
                execution_kind: "static".into(),
                ci_jobs: vec!["web-job".into()],
                producer_ids: vec!["ci.web".into()],
                checks: vec!["cargo.web".into()],
                artifact_inputs: vec!["source".into()],
                isolation: isolation(),
            },
            ImpactShard {
                shard_id: "runtime-shard".into(),
                layer: "runtime".into(),
                execution_kind: "application".into(),
                ci_jobs: Vec::new(),
                producer_ids: vec!["ci.runtime".into()],
                checks: vec!["runtime.check".into()],
                artifact_inputs: vec!["source".into()],
                isolation: isolation(),
            },
        ],
    }
}

fn evidence_catalog() -> BTreeMap<String, Vec<String>> {
    BTreeMap::from([
        ("ci.core".into(), vec!["test.core".into()]),
        ("ci.web".into(), vec!["test.web".into()]),
        ("ci.runtime".into(), vec!["smoke.runtime".into()]),
    ])
}

fn fingerprints() -> InputFingerprints {
    InputFingerprints {
        impact_registry: "sha256:impact".into(),
        producer_registry: "sha256:producer".into(),
        acceptance_matrix: "sha256:matrix".into(),
    }
}

#[test]
fn reverse_dependency_closure_includes_transitive_consumers() {
    let modules = vec![
        module("core", &[]),
        module("cli", &["core"]),
        module("desktop", &["cli"]),
    ];
    let changed = BTreeSet::from(["core"]);
    assert_eq!(
        reverse_closure(&changed, &modules),
        BTreeSet::from(["cli", "core", "desktop"])
    );
}

#[test]
fn inputs_reject_shell_options_and_noncanonical_paths() {
    assert!(input::validate_revision("main").is_ok());
    assert!(input::validate_revision("-C").is_err());
    assert!(input::validate_changed_path("apps/web/src/lib.rs").is_ok());
    assert!(input::validate_changed_path("apps/web/../cli/lib.rs").is_err());
}

#[test]
fn selective_plan_expands_consumers_and_unknown_paths_force_full() {
    let registry = planning_registry();
    let args = PlanArgs::parse(
        &["--profile", "pr-selective", "--changed-file", "core/lib.rs"].map(str::to_owned),
    )
    .unwrap();
    let plan = build(
        Path::new("."),
        &registry,
        fingerprints(),
        args,
        &evidence_catalog(),
        &[],
    )
    .unwrap();
    assert_eq!(plan.selection, "selective");
    assert_eq!(plan.scope, "source");
    assert_eq!(plan.reverse_consumers, vec!["web"]);
    assert_eq!(plan.selected_shards, vec!["core-shard", "web-shard"]);

    let args = PlanArgs::parse(
        &[
            "--profile",
            "pr-selective",
            "--changed-file",
            "unknown/file",
        ]
        .map(str::to_owned),
    )
    .unwrap();
    let plan = build(
        Path::new("."),
        &registry,
        fingerprints(),
        args,
        &evidence_catalog(),
        &[],
    )
    .unwrap();
    assert_eq!(plan.status, "shadow-only");
    assert_eq!(plan.selection, "full");
    assert_eq!(plan.scope, "system");
    assert!(plan.selected_shards.contains(&"runtime-shard".to_owned()));
    assert_eq!(plan.full_reasons, vec!["unknown-path:unknown/file"]);
}

#[test]
fn candidate_profile_is_full_without_a_change_set() {
    let registry = planning_registry();
    let args =
        PlanArgs::parse(&["--profile", "candidate-full-release"].map(str::to_owned)).unwrap();
    let plan = build(
        Path::new("."),
        &registry,
        fingerprints(),
        args,
        &evidence_catalog(),
        &[],
    )
    .unwrap();
    assert_eq!(plan.selection, "full");
    assert!(plan.reverse_consumers.is_empty());
    assert_eq!(plan.selected_modules, vec!["core", "web"]);
}

#[test]
fn unknown_or_full_trigger_paths_escalate_source_profiles_to_system_scope() {
    let registry = planning_registry();
    let args = PlanArgs::parse(
        &[
            "--profile",
            "main-full-source",
            "--changed-file",
            "unknown/file",
        ]
        .map(str::to_owned),
    )
    .unwrap();
    let plan = build(
        Path::new("."),
        &registry,
        fingerprints(),
        args,
        &evidence_catalog(),
        &[],
    )
    .unwrap();
    assert_eq!(plan.scope, "system");
    assert!(plan.selected_shards.contains(&"runtime-shard".to_owned()));

    let mut registry = planning_registry();
    registry.modules[0].full_trigger = true;
    let args = PlanArgs::parse(
        &[
            "--profile",
            "main-full-source",
            "--changed-file",
            "core/lib.rs",
        ]
        .map(str::to_owned),
    )
    .unwrap();
    let plan = build(
        Path::new("."),
        &registry,
        fingerprints(),
        args,
        &evidence_catalog(),
        &[],
    )
    .unwrap();
    assert_eq!(plan.scope, "system");
    assert!(plan.selected_shards.contains(&"runtime-shard".to_owned()));
}
