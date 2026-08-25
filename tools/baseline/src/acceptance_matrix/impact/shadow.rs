//! Full-run observation report for the shadow-only PR selector.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::model::{ImpactPlan, ImpactRegistry, InputFingerprints};
use super::plan::{self, PlanArgs};
use crate::acceptance_matrix::model::MatrixRow;
use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Serialize)]
struct ShadowReport {
    schema: u8,
    status: &'static str,
    selector_outcome: &'static str,
    selection: ImpactPlan,
    full_job_results: BTreeMap<String, String>,
    selected_ci_jobs: Vec<String>,
    full_failures: Vec<String>,
    observed_misses: Vec<String>,
}

struct Args {
    base: String,
    head: String,
    results: BTreeMap<String, String>,
}

pub(super) fn render(
    root: &Path,
    registry: &ImpactRegistry,
    fingerprints: InputFingerprints,
    args: &[String],
    evidence_by_producer: &BTreeMap<String, Vec<String>>,
    rows: &[MatrixRow],
) -> Result<String> {
    let args = parse_args(args)?;
    let selection_args = PlanArgs::parse(&[
        "--profile".into(),
        "pr-selective".into(),
        "--base".into(),
        args.base,
        "--head".into(),
        args.head,
    ])?;
    let selection = plan::build(
        root,
        registry,
        fingerprints,
        selection_args,
        evidence_by_producer,
        rows,
    )?;
    let shard_by_id = registry
        .shards
        .iter()
        .map(|shard| (shard.shard_id.as_str(), shard))
        .collect::<BTreeMap<_, _>>();
    let required_jobs = registry
        .shards
        .iter()
        .filter(|shard| shard.layer == "source")
        .flat_map(|shard| shard.ci_jobs.iter().cloned())
        .collect::<BTreeSet<_>>();
    let supplied_jobs = args.results.keys().cloned().collect::<BTreeSet<_>>();
    validate_result_set(&supplied_jobs, &required_jobs)?;
    let selected_jobs = selection
        .selected_shards
        .iter()
        .flat_map(|shard_id| {
            shard_by_id
                .get(shard_id.as_str())
                .into_iter()
                .flat_map(|shard| shard.ci_jobs.iter().cloned())
        })
        .collect::<BTreeSet<_>>();
    let (full_failures, observed_misses) = classify_results(&args.results, &selected_jobs);
    serde_json::to_string_pretty(&ShadowReport {
        schema: 1,
        status: "shadow-only-not-pass-evidence",
        selector_outcome: if observed_misses.is_empty() {
            "no-observed-miss"
        } else {
            "observed-miss"
        },
        selection,
        full_job_results: args.results,
        selected_ci_jobs: selected_jobs.into_iter().collect(),
        full_failures,
        observed_misses,
    })
    .map_err(Into::into)
}

fn validate_result_set(supplied: &BTreeSet<String>, required: &BTreeSet<String>) -> Result<()> {
    if supplied != required {
        bail!(
            "acceptance-impact-shadow: full CI result set is incomplete or contains an unknown job"
        );
    }
    Ok(())
}

fn classify_results(
    results: &BTreeMap<String, String>,
    selected_jobs: &BTreeSet<String>,
) -> (Vec<String>, Vec<String>) {
    let full_failures = results
        .iter()
        .filter(|(_, status)| status.as_str() != "success")
        .map(|(job, _)| job.clone())
        .collect::<Vec<_>>();
    let observed_misses = full_failures
        .iter()
        .filter(|job| !selected_jobs.contains(*job))
        .cloned()
        .collect::<Vec<_>>();
    (full_failures, observed_misses)
}

fn parse_args(args: &[String]) -> Result<Args> {
    let mut base = None;
    let mut head = None;
    let mut results = BTreeMap::new();
    let mut index = 0;
    while index < args.len() {
        let option = args[index].as_str();
        let value = args
            .get(index + 1)
            .with_context(|| format!("acceptance-impact-shadow: {option} requires a value"))?;
        index += 2;
        match option {
            "--base" if base.is_none() => base = Some(plan::input::validate_revision(value)?),
            "--head" if head.is_none() => head = Some(plan::input::validate_revision(value)?),
            "--result" => {
                let (job, status) = value
                    .split_once('=')
                    .context("acceptance-impact-shadow: --result must be job=status")?;
                if job.is_empty()
                    || !job
                        .chars()
                        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
                    || !matches!(status, "success" | "failure" | "cancelled" | "skipped")
                    || results.insert(job.to_owned(), status.to_owned()).is_some()
                {
                    bail!("acceptance-impact-shadow: invalid or duplicate full CI result");
                }
            }
            _ => bail!("acceptance-impact-shadow: unknown or repeated option {option}"),
        }
    }
    if results.is_empty() {
        bail!("acceptance-impact-shadow: at least one full CI result is required");
    }
    Ok(Args {
        base: base.context("acceptance-impact-shadow: --base is required")?,
        head: head.context("acceptance-impact-shadow: --head is required")?,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::{classify_results, parse_args, validate_result_set};
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn results_are_typed_and_duplicate_jobs_fail_closed() {
        assert!(
            parse_args(&[
                "--base".into(),
                "base".into(),
                "--head".into(),
                "head".into(),
                "--result".into(),
                "rust-quality=success".into(),
            ])
            .is_ok()
        );
        assert!(
            parse_args(&[
                "--base".into(),
                "base".into(),
                "--head".into(),
                "head".into(),
                "--result".into(),
                "rust-quality=success".into(),
                "--result".into(),
                "rust-quality=skipped".into(),
            ])
            .is_err()
        );
    }

    #[test]
    fn complete_results_preserve_every_non_success_and_observed_miss() {
        let required = BTreeSet::from(["a".to_owned(), "b".to_owned(), "c".to_owned()]);
        assert!(
            validate_result_set(&BTreeSet::from(["a".to_owned(), "b".to_owned()]), &required)
                .is_err()
        );
        assert!(
            validate_result_set(
                &BTreeSet::from([
                    "a".to_owned(),
                    "b".to_owned(),
                    "c".to_owned(),
                    "unknown".to_owned()
                ]),
                &required
            )
            .is_err()
        );
        validate_result_set(&required, &required).unwrap();

        let results = BTreeMap::from([
            ("a".to_owned(), "failure".to_owned()),
            ("b".to_owned(), "cancelled".to_owned()),
            ("c".to_owned(), "skipped".to_owned()),
            ("d".to_owned(), "success".to_owned()),
        ]);
        let selected = BTreeSet::from(["a".to_owned()]);
        let (failures, misses) = classify_results(&results, &selected);
        assert_eq!(failures, ["a", "b", "c"]);
        assert_eq!(misses, ["b", "c"]);
    }
}
