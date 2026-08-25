//! Verification shard, producer ownership, and runtime isolation validation.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::super::model::{ImpactRegistry, ImpactShard, Isolation};
use anyhow::{Result, bail};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate(
    registry: &ImpactRegistry,
    artifacts: &BTreeSet<&str>,
    producer_ids: &BTreeSet<String>,
    ci_job_by_producer: &BTreeMap<String, String>,
) -> Result<()> {
    let required_ci_jobs = BTreeSet::from([
        "contract-checks",
        "rust-quality",
        "workspace-tests",
        "ci-acceptance-runtime",
        "ci-acceptance-storage",
        "ci-acceptance-web",
        "ci-acceptance-windows",
        "watcher-native-fs",
    ]);
    let mut mapped_ci_jobs = BTreeSet::new();
    let known_producers = producer_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut assigned = BTreeSet::new();
    for shard in &registry.shards {
        let ci_jobs = super::unique_values(&shard.ci_jobs, "shard CI job")?;
        if shard.layer == "source" && ci_jobs.is_empty() {
            bail!(
                "acceptance-impact: source shard {} has no full CI job mapping",
                shard.shard_id
            );
        }
        if shard.layer == "runtime" && !ci_jobs.is_empty() {
            bail!(
                "acceptance-impact: runtime shard {} must not claim ordinary PR CI jobs",
                shard.shard_id
            );
        }
        if !ci_jobs.is_subset(&required_ci_jobs) {
            bail!(
                "acceptance-impact: shard {} references an unknown full CI job",
                shard.shard_id
            );
        }
        mapped_ci_jobs.extend(ci_jobs.iter().copied());
        let producers = super::unique_values(&shard.producer_ids, "shard producer")?;
        validate_layer_producer_jobs(shard, &producers, ci_job_by_producer)?;
        let actual_ci_jobs = producers
            .iter()
            .filter_map(|producer| ci_job_by_producer.get(*producer).map(String::as_str))
            .collect::<BTreeSet<_>>();
        if !actual_ci_jobs.is_subset(&ci_jobs) {
            bail!(
                "acceptance-impact: shard {} omits an actual CI producer job",
                shard.shard_id
            );
        }
        let checks = super::unique_values(&shard.checks, "shard check")?;
        if producers.is_empty() && checks.is_empty() {
            bail!(
                "acceptance-impact: shard {} has no verification unit",
                shard.shard_id
            );
        }
        if !producers.is_subset(&known_producers) {
            bail!(
                "acceptance-impact: shard {} references an unknown producer",
                shard.shard_id
            );
        }
        for producer in producers {
            if !assigned.insert(producer) {
                bail!("acceptance-impact: producer {producer} is assigned to multiple shards");
            }
        }
        let inputs = super::unique_values(&shard.artifact_inputs, "artifact input")?;
        if inputs.is_empty() || !inputs.is_subset(artifacts) {
            bail!(
                "acceptance-impact: shard {} has an invalid artifact input",
                shard.shard_id
            );
        }
        validate_isolation(shard)?;
    }
    if assigned != known_producers {
        bail!("acceptance-impact: every acceptance producer must belong to exactly one shard");
    }
    if mapped_ci_jobs != required_ci_jobs {
        bail!("acceptance-impact: source shard CI job mapping is incomplete");
    }
    validate_fixed_base_job_bindings(registry)?;
    Ok(())
}

fn validate_layer_producer_jobs(
    shard: &ImpactShard,
    producers: &BTreeSet<&str>,
    ci_job_by_producer: &BTreeMap<String, String>,
) -> Result<()> {
    let ci_producers = producers
        .iter()
        .filter(|producer| ci_job_by_producer.contains_key(**producer))
        .copied()
        .collect::<BTreeSet<_>>();
    if shard.layer == "source" && ci_producers != *producers {
        bail!(
            "acceptance-impact: source shard {} may contain only producers executed by full CI",
            shard.shard_id
        );
    }
    if shard.layer == "runtime" && !ci_producers.is_empty() {
        bail!(
            "acceptance-impact: runtime shard {} may not contain ordinary CI producers",
            shard.shard_id
        );
    }
    Ok(())
}

fn validate_fixed_base_job_bindings(registry: &ImpactRegistry) -> Result<()> {
    let required = BTreeMap::from([
        ("contract-static", BTreeSet::from(["contract-checks"])),
        (
            "workspace-build",
            BTreeSet::from(["rust-quality", "workspace-tests"]),
        ),
        ("core-ci", BTreeSet::from(["watcher-native-fs"])),
    ]);
    let shards = registry
        .shards
        .iter()
        .map(|shard| (shard.shard_id.as_str(), shard))
        .collect::<BTreeMap<_, _>>();
    for (shard_id, required_jobs) in required {
        let shard = shards.get(shard_id).ok_or_else(|| {
            anyhow::anyhow!("acceptance-impact: missing fixed source shard {shard_id}")
        })?;
        let actual = shard
            .ci_jobs
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if !required_jobs.is_subset(&actual) {
            bail!("acceptance-impact: fixed source shard {shard_id} omits required base CI jobs");
        }
    }
    Ok(())
}

fn validate_isolation(shard: &ImpactShard) -> Result<()> {
    if !matches!(
        shard.execution_kind.as_str(),
        "static" | "process" | "application"
    ) {
        bail!(
            "acceptance-impact: shard {} has an invalid execution kind",
            shard.shard_id
        );
    }
    let isolation = &shard.isolation;
    let scoped = |value: &str| matches!(value, "not-applicable" | "per-execution" | "per-job");
    if !scoped(&isolation.state)
        || !matches!(isolation.ports.as_str(), "not-applicable" | "allocated")
        || !scoped(&isolation.database)
        || !scoped(&isolation.identity)
        || !matches!(
            isolation.fixture.as_str(),
            "not-applicable" | "per-execution" | "per-job" | "content-addressed"
        )
        || !scoped(&isolation.logs)
    {
        bail!(
            "acceptance-impact: shard {} has an invalid dimension-typed isolation policy",
            shard.shard_id
        );
    }
    let missing_required_isolation = match shard.execution_kind.as_str() {
        "application" => isolation_values(&shard.isolation).contains(&"not-applicable"),
        "process" => [
            shard.isolation.state.as_str(),
            shard.isolation.database.as_str(),
            shard.isolation.identity.as_str(),
            shard.isolation.fixture.as_str(),
            shard.isolation.logs.as_str(),
        ]
        .contains(&"not-applicable"),
        "static" => false,
        _ => unreachable!("execution kind was validated above"),
    };
    if missing_required_isolation {
        let resources = if shard.execution_kind == "application" {
            "state, ports, database, identity, fixture, and logs"
        } else {
            "state, database, identity, fixture, and logs"
        };
        bail!(
            "acceptance-impact: {} shard {} must isolate {resources}",
            shard.execution_kind,
            shard.shard_id
        );
    }
    Ok(())
}

fn isolation_values(isolation: &Isolation) -> [&str; 6] {
    [
        &isolation.state,
        &isolation.ports,
        &isolation.database,
        &isolation.identity,
        &isolation.fixture,
        &isolation.logs,
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        validate_fixed_base_job_bindings, validate_isolation, validate_layer_producer_jobs,
    };
    use crate::acceptance_matrix::impact::model::{ImpactShard, Isolation};
    use std::collections::{BTreeMap, BTreeSet};

    fn shard(kind: &str, isolation: Isolation) -> ImpactShard {
        ImpactShard {
            shard_id: "fixture".into(),
            layer: "source".into(),
            execution_kind: kind.into(),
            ci_jobs: vec!["contract-checks".into()],
            producer_ids: Vec::new(),
            checks: vec!["fixture".into()],
            artifact_inputs: vec!["source".into()],
            isolation,
        }
    }

    #[test]
    fn application_and_process_shards_reject_nonisolated_fields() {
        let nonisolated = Isolation {
            state: "per-job".into(),
            ports: "not-applicable".into(),
            database: "per-job".into(),
            identity: "per-job".into(),
            fixture: "content-addressed".into(),
            logs: "per-job".into(),
        };
        assert!(validate_isolation(&shard("application", nonisolated.clone())).is_err());
        assert!(validate_isolation(&shard("process", nonisolated.clone())).is_ok());
        assert!(validate_isolation(&shard("static", nonisolated)).is_ok());

        let missing_state = Isolation {
            state: "not-applicable".into(),
            ports: "allocated".into(),
            database: "per-job".into(),
            identity: "per-job".into(),
            fixture: "content-addressed".into(),
            logs: "per-job".into(),
        };
        assert!(validate_isolation(&shard("process", missing_state)).is_err());
    }

    #[test]
    fn isolation_values_are_typed_per_resource_dimension() {
        let invalid_port = Isolation {
            state: "per-job".into(),
            ports: "content-addressed".into(),
            database: "per-job".into(),
            identity: "per-job".into(),
            fixture: "content-addressed".into(),
            logs: "per-job".into(),
        };
        assert!(validate_isolation(&shard("application", invalid_port)).is_err());

        let invalid_fixture = Isolation {
            state: "not-applicable".into(),
            ports: "not-applicable".into(),
            database: "not-applicable".into(),
            identity: "not-applicable".into(),
            fixture: "allocated".into(),
            logs: "not-applicable".into(),
        };
        assert!(validate_isolation(&shard("static", invalid_fixture)).is_err());
    }

    #[test]
    fn fixed_base_jobs_cannot_move_to_an_optional_source_shard() {
        let isolation = Isolation {
            state: "per-job".into(),
            ports: "not-applicable".into(),
            database: "per-job".into(),
            identity: "per-job".into(),
            fixture: "content-addressed".into(),
            logs: "per-job".into(),
        };
        let mut registry = crate::acceptance_matrix::impact::model::ImpactRegistry {
            schema: 1,
            mode: "shadow-only".into(),
            artifacts: Vec::new(),
            profiles: Vec::new(),
            modules: Vec::new(),
            shards: vec![
                shard("static", isolation.clone()),
                shard("static", isolation.clone()),
                shard("static", isolation.clone()),
            ],
        };
        registry.shards[0].shard_id = "contract-static".into();
        registry.shards[0].ci_jobs = vec!["contract-checks".into()];
        registry.shards[1].shard_id = "workspace-build".into();
        registry.shards[1].ci_jobs = vec!["rust-quality".into(), "workspace-tests".into()];
        registry.shards[2].shard_id = "core-ci".into();
        registry.shards[2].ci_jobs = vec!["watcher-native-fs".into()];
        validate_fixed_base_job_bindings(&registry).unwrap();
        registry.shards[2].ci_jobs.clear();
        assert!(validate_fixed_base_job_bindings(&registry).is_err());
    }

    #[test]
    fn source_shards_reject_target_host_only_producers() {
        let isolation = Isolation {
            state: "per-job".into(),
            ports: "not-applicable".into(),
            database: "per-job".into(),
            identity: "per-job".into(),
            fixture: "content-addressed".into(),
            logs: "per-job".into(),
        };
        let source = shard("static", isolation.clone());
        let producers = BTreeSet::from(["target-host.only"]);
        assert!(validate_layer_producer_jobs(&source, &producers, &BTreeMap::new()).is_err());
        let mut runtime = shard("process", isolation);
        runtime.layer = "runtime".into();
        assert!(validate_layer_producer_jobs(&runtime, &producers, &BTreeMap::new()).is_ok());
    }
}
