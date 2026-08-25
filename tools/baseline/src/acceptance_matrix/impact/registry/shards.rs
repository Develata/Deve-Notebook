//! Verification shard, producer ownership, and runtime isolation validation.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::super::model::{ImpactRegistry, ImpactShard, Isolation};
use anyhow::{Result, bail};
use std::collections::BTreeSet;

pub(super) fn validate(
    registry: &ImpactRegistry,
    artifacts: &BTreeSet<&str>,
    producer_ids: &BTreeSet<String>,
) -> Result<()> {
    let known_producers = producer_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut assigned = BTreeSet::new();
    for shard in &registry.shards {
        let producers = super::unique_values(&shard.producer_ids, "shard producer")?;
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
    use super::validate_isolation;
    use crate::acceptance_matrix::impact::model::{ImpactShard, Isolation};

    fn shard(kind: &str, isolation: Isolation) -> ImpactShard {
        ImpactShard {
            shard_id: "fixture".into(),
            layer: "source".into(),
            execution_kind: kind.into(),
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
}
