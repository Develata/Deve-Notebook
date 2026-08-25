//! Pure producer selection, host compatibility, and clean-tree preflight.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::model::{Producer, ProducerRegistry};
use crate::acceptance_matrix::model::MatrixRow;
use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub(super) struct ProducerPlan<'a> {
    pub(super) producer: &'a Producer,
    pub(super) evidence: Vec<&'a MatrixRow>,
    pub(super) selected: bool,
    host_supported: bool,
    tag_ready_host_supported: bool,
    missing_env: Vec<String>,
}

impl ProducerPlan<'_> {
    pub(super) fn emits_receipts(&self) -> bool {
        self.evidence
            .iter()
            .any(|row| row.evidence_kind == "receipt")
    }
}

pub(super) fn build_plan<'a>(
    args: &RunArgs,
    registry: &'a ProducerRegistry,
    rows: &'a [MatrixRow],
) -> Result<Vec<ProducerPlan<'a>>> {
    validate_filters(args, registry, rows)?;
    let evidence_rows = executable_evidence_rows(rows);
    let mut plans = Vec::new();
    for producer in &registry.producers {
        if !producer.tiers.contains(&args.tier) {
            continue;
        }
        let producer_selected =
            args.producers.is_empty() || args.producers.contains(producer.producer_id.as_str());
        let mut evidence = Vec::new();
        for evidence_id in &producer.evidence_ids {
            if let Some(row) = evidence_rows.get(evidence_id.as_str()) {
                evidence.push(*row);
            }
        }
        // A producer execution and its receipt group are atomic. Evidence
        // filters select the owning producer, but may never split the
        // producer's declared evidence set into a partial execution group.
        let evidence_selected = args.evidence_ids.is_empty()
            || producer
                .evidence_ids
                .iter()
                .any(|evidence_id| args.evidence_ids.contains(evidence_id.as_str()));
        let selected = producer_selected && evidence_selected;
        let host_supported = producer
            .host_os
            .iter()
            .any(|host| host == std::env::consts::OS);
        let tag_ready_host_supported = args.tier != "tag-ready"
            || evidence
                .iter()
                .all(|row| tag_ready_host_supports(row, std::env::consts::OS));
        let missing_env = producer
            .required_env
            .iter()
            .filter(|name| std::env::var_os(name).is_none_or(|value| value.is_empty()))
            .cloned()
            .collect();
        plans.push(ProducerPlan {
            producer,
            evidence,
            selected,
            host_supported,
            tag_ready_host_supported,
            missing_env,
        });
    }
    if plans.is_empty() {
        bail!(
            "acceptance-run: tier {} has no registered producers",
            args.tier
        );
    }
    let producers = registry
        .producers
        .iter()
        .map(|producer| (producer.producer_id.as_str(), producer))
        .collect::<BTreeMap<_, _>>();
    plans.sort_by(|left, right| {
        dependency_depth(left.producer, &producers)
            .cmp(&dependency_depth(right.producer, &producers))
            .then_with(|| left.producer.producer_id.cmp(&right.producer.producer_id))
    });
    let selected = plans
        .iter()
        .filter(|plan| plan.selected)
        .map(|plan| plan.producer.producer_id.as_str())
        .collect::<BTreeSet<_>>();
    if selected.is_empty() {
        bail!(
            "acceptance-run: filters selected no producers for tier {}",
            args.tier
        );
    }
    for plan in plans.iter().filter(|plan| plan.selected) {
        for dependency in &plan.producer.dependencies {
            if !selected.contains(dependency.as_str()) {
                bail!(
                    "acceptance-run: selected producer {} requires dependency {dependency}; filters may not create a partial dependency plan",
                    plan.producer.producer_id
                );
            }
        }
    }
    Ok(plans)
}

fn dependency_depth<'a>(
    producer: &'a Producer,
    producers: &BTreeMap<&'a str, &'a Producer>,
) -> usize {
    producer
        .dependencies
        .iter()
        .filter_map(|dependency| producers.get(dependency.as_str()))
        .map(|dependency| dependency_depth(dependency, producers) + 1)
        .max()
        .unwrap_or(0)
}

fn validate_filters(args: &RunArgs, registry: &ProducerRegistry, rows: &[MatrixRow]) -> Result<()> {
    let producers: BTreeSet<_> = registry
        .producers
        .iter()
        .map(|producer| producer.producer_id.as_str())
        .collect();
    for producer in &args.producers {
        if !producers.contains(producer.as_str()) {
            bail!("acceptance-run: unknown producer {producer}");
        }
    }
    let evidence: BTreeSet<_> = rows
        .iter()
        .filter(|row| matches!(row.evidence_kind.as_str(), "test" | "script" | "receipt"))
        .map(|row| row.evidence_id.as_str())
        .collect();
    for evidence_id in &args.evidence_ids {
        if !evidence.contains(evidence_id.as_str()) {
            bail!("acceptance-run: unknown executable evidence {evidence_id}");
        }
    }
    Ok(())
}

pub(super) fn print_plan(args: &RunArgs, plans: &[ProducerPlan<'_>]) {
    println!(
        "acceptance-run plan: tier={} host={} mode={}",
        args.tier,
        std::env::consts::OS,
        if args.plan { "plan" } else { "execute" }
    );
    for plan in plans {
        let status = if !plan.selected {
            "filtered"
        } else if !plan.host_supported {
            "host-unavailable"
        } else if !plan.tag_ready_host_supported {
            "tag-ready-host-mismatch"
        } else if !plan.missing_env.is_empty() {
            "missing-env"
        } else {
            "ready"
        };
        println!(
            "- {} status={} evidence={}{}",
            plan.producer.producer_id,
            status,
            plan.evidence
                .iter()
                .map(|row| row.evidence_id.as_str())
                .collect::<Vec<_>>()
                .join(","),
            if plan.missing_env.is_empty() {
                String::new()
            } else {
                format!(" missing_env={}", plan.missing_env.join(","))
            }
        );
    }
    let selected = plans.iter().filter(|plan| plan.selected).count();
    let ready = plans
        .iter()
        .filter(|plan| {
            plan.selected
                && plan.host_supported
                && plan.tag_ready_host_supported
                && plan.missing_env.is_empty()
        })
        .count();
    println!(
        "acceptance-run plan summary: selected={selected} ready={ready} unavailable={}",
        selected.saturating_sub(ready)
    );
}

pub(super) fn preflight_execution(
    root: &Path,
    args: &RunArgs,
    plans: &[ProducerPlan<'_>],
) -> Result<()> {
    if !git_status(root)?.is_empty() {
        bail!("acceptance-run: worktree must be clean before execution");
    }
    let selected: Vec<_> = plans.iter().filter(|plan| plan.selected).collect();
    if selected.is_empty() {
        bail!(
            "acceptance-run: filters selected no producers for tier {}",
            args.tier
        );
    }
    for plan in selected {
        if !plan.host_supported {
            bail!(
                "acceptance-run: producer {} does not support host {}",
                plan.producer.producer_id,
                std::env::consts::OS
            );
        }
        if !plan.tag_ready_host_supported {
            bail!(
                "acceptance-run: producer {} on {} cannot satisfy tag-ready platform constraints",
                plan.producer.producer_id,
                std::env::consts::OS
            );
        }
        if !plan.missing_env.is_empty() {
            bail!(
                "acceptance-run: producer {} is missing required environment: {}",
                plan.producer.producer_id,
                plan.missing_env.join(", ")
            );
        }
    }
    Ok(())
}

fn executable_evidence_rows(rows: &[MatrixRow]) -> BTreeMap<&str, &MatrixRow> {
    rows.iter()
        .filter(|row| matches!(row.evidence_kind.as_str(), "test" | "script" | "receipt"))
        .map(|row| (row.evidence_id.as_str(), row))
        .collect()
}

fn tag_ready_host_supports(row: &MatrixRow, host: &str) -> bool {
    match row.surface.as_str() {
        "docker" => host == "linux",
        "desktop" => host == "windows",
        _ => true,
    }
}

pub(super) fn git_status(root: &Path) -> Result<String> {
    git_output(
        root,
        [
            "-c",
            "status.showUntrackedFiles=all",
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
            "--ignore-submodules=none",
        ],
    )
}

pub(super) fn git_output<const N: usize>(root: &Path, args: [&str; N]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .context("acceptance-run: failed to run git")?;
    if !output.status.success() {
        bail!("acceptance-run: git command failed");
    }
    String::from_utf8(output.stdout)
        .context("acceptance-run: git output was not UTF-8")
        .map(|value| value.trim().to_string())
}

#[derive(Debug)]
pub(super) struct RunArgs {
    pub(super) tier: String,
    pub(super) plan: bool,
    pub(super) receipt_dir: Option<PathBuf>,
    producers: BTreeSet<String>,
    evidence_ids: BTreeSet<String>,
}

impl RunArgs {
    pub(super) fn parse(args: &[String]) -> Result<Self> {
        let mut tier = None;
        let mut plan = false;
        let mut receipt_dir = None;
        let mut producers = BTreeSet::new();
        let mut evidence_ids = BTreeSet::new();
        let mut index = 0usize;
        while index < args.len() {
            match args[index].as_str() {
                "--plan" => {
                    plan = true;
                    index += 1;
                }
                "--tier" | "--receipt-dir" | "--producer" | "--evidence-id" => {
                    let option = &args[index];
                    let value = args
                        .get(index + 1)
                        .with_context(|| format!("acceptance-run: missing value for {option}"))?;
                    match option.as_str() {
                        "--tier" => tier = Some(value.clone()),
                        "--receipt-dir" => receipt_dir = Some(PathBuf::from(value)),
                        "--producer" => {
                            producers.insert(value.clone());
                        }
                        "--evidence-id" => {
                            evidence_ids.insert(value.clone());
                        }
                        _ => unreachable!(),
                    }
                    index += 2;
                }
                other => bail!("acceptance-run: unknown option {other}"),
            }
        }
        let tier = tier.context("acceptance-run: --tier is required")?;
        if !matches!(tier.as_str(), "ci" | "full" | "target-host" | "tag-ready") {
            bail!("acceptance-run: unsupported tier {tier}");
        }
        if plan && receipt_dir.is_some() {
            bail!("acceptance-run: --plan may not be combined with --receipt-dir");
        }
        Ok(Self {
            tier,
            plan,
            receipt_dir,
            producers,
            evidence_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{RunArgs, build_plan};
    use crate::acceptance_matrix::model::MatrixRow;
    use crate::acceptance_matrix::producer::model::{Producer, ProducerRegistry};
    use std::collections::BTreeMap;

    #[test]
    fn run_args_support_repeatable_narrowing_filters() {
        let args = [
            "--tier",
            "target-host",
            "--plan",
            "--producer",
            "android.lifecycle",
            "--evidence-id",
            "smoke.android.local-backend",
        ]
        .map(str::to_string);
        let parsed = RunArgs::parse(&args).unwrap();
        assert!(parsed.plan);
        assert!(parsed.producers.contains("android.lifecycle"));
        assert!(parsed.evidence_ids.contains("smoke.android.local-backend"));
    }

    #[test]
    fn plan_rejects_receipt_output() {
        let args = ["--tier", "ci", "--plan", "--receipt-dir", "out"].map(str::to_string);
        assert!(RunArgs::parse(&args).is_err());
    }

    #[test]
    fn evidence_filter_selects_the_complete_atomic_producer_group() {
        let args = ["--tier", "full", "--plan", "--evidence-id", "smoke.one"].map(str::to_string);
        let args = RunArgs::parse(&args).unwrap();
        let registry = ProducerRegistry {
            schema: 2,
            producers: vec![Producer {
                producer_id: "smoke.group".into(),
                evidence_ids: vec!["smoke.one".into(), "smoke.two".into()],
                dependencies: Vec::new(),
                tiers: vec!["full".into()],
                host_os: vec![std::env::consts::OS.into()],
                timeout_seconds: 1,
                required_tools: Vec::new(),
                required_env: Vec::new(),
                bound_env: Vec::new(),
                environment: BTreeMap::new(),
                claims_env: BTreeMap::new(),
                artifacts: Vec::new(),
                steps: Vec::new(),
                finally_steps: Vec::new(),
                note: "fixture".into(),
            }],
        };
        let rows = ["smoke.one", "smoke.two"].map(|evidence_id| MatrixRow {
            requirement_id: format!("requirement.{evidence_id}"),
            journey_id: "fixture".into(),
            flow_id: "none".into(),
            case_id: "none".into(),
            surface: "web".into(),
            mode: "browser".into(),
            gate: "tag-ready".into(),
            requirement: "required".into(),
            evidence_kind: "receipt".into(),
            evidence_id: evidence_id.into(),
            evidence_ref: format!("receipts/{evidence_id}.json"),
            freshness: "current-head".into(),
            note: String::new(),
        });

        let plans = build_plan(&args, &registry, &rows).unwrap();

        assert!(plans[0].selected);
        assert_eq!(
            plans[0]
                .evidence
                .iter()
                .map(|row| row.evidence_id.as_str())
                .collect::<Vec<_>>(),
            vec!["smoke.one", "smoke.two"]
        );
    }

    #[test]
    fn producer_filter_may_not_cut_a_declared_dependency() {
        let args = ["--tier", "ci", "--plan", "--producer", "ci.child"].map(str::to_string);
        let args = RunArgs::parse(&args).unwrap();
        let producer = |producer_id: &str, evidence_id: &str, dependencies: Vec<String>| Producer {
            producer_id: producer_id.into(),
            evidence_ids: vec![evidence_id.into()],
            dependencies,
            tiers: vec!["ci".into()],
            host_os: vec![std::env::consts::OS.into()],
            timeout_seconds: 1,
            required_tools: Vec::new(),
            required_env: Vec::new(),
            bound_env: Vec::new(),
            environment: BTreeMap::new(),
            claims_env: BTreeMap::new(),
            artifacts: Vec::new(),
            steps: Vec::new(),
            finally_steps: Vec::new(),
            note: "fixture".into(),
        };
        let registry = ProducerRegistry {
            schema: 2,
            producers: vec![
                producer("ci.parent", "test.parent", Vec::new()),
                producer("ci.child", "test.child", vec!["ci.parent".into()]),
            ],
        };
        let rows = ["test.parent", "test.child"].map(|evidence_id| MatrixRow {
            requirement_id: format!("requirement.{evidence_id}"),
            journey_id: "none".into(),
            flow_id: "none".into(),
            case_id: "none".into(),
            surface: "core".into(),
            mode: "integration".into(),
            gate: "ci".into(),
            requirement: "required".into(),
            evidence_kind: "test".into(),
            evidence_id: evidence_id.into(),
            evidence_ref: "cargo test -p deve_core fixture -- --nocapture".into(),
            freshness: "source-bound".into(),
            note: String::new(),
        });

        let error = build_plan(&args, &registry, &rows).unwrap_err();

        assert!(error.to_string().contains("requires dependency ci.parent"));
    }
}
