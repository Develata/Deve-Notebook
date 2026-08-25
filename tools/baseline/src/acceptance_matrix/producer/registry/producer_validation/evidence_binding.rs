//! Direct producer-step binding for executable acceptance evidence.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use crate::acceptance_matrix::model::MatrixRow;
use crate::acceptance_matrix::producer::model::{ProducerArg, ProducerRegistry, ProducerStep};
use crate::acceptance_matrix::test_selector::TestSelector;
use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(in crate::acceptance_matrix::producer) fn executable_evidence_ids(
    registry: &ProducerRegistry,
    matrix_evidence: &BTreeMap<&str, &MatrixRow>,
) -> Result<BTreeSet<String>> {
    let mut bound = BTreeSet::new();
    for producer in &registry.producers {
        for evidence_id in &producer.evidence_ids {
            let row = matrix_evidence
                .get(evidence_id.as_str())
                .with_context(|| format!("acceptance producers: missing evidence {evidence_id}"))?;
            let mut directly_executed = row.evidence_kind == "receipt"
                && (producer.candidate_required
                    || producer.tiers.iter().any(|tier| tier == "target-host"));
            for step in &producer.steps {
                directly_executed |= step_directly_executes(row, step)?;
            }
            if directly_executed {
                bound.insert(evidence_id.clone());
            }
        }
    }
    Ok(bound)
}

fn step_directly_executes(row: &MatrixRow, step: &ProducerStep) -> Result<bool> {
    let literal_args = step
        .args
        .iter()
        .map(|arg| match arg {
            ProducerArg::LiteralString(value) => Some(value.as_str()),
            ProducerArg::Literal { literal } => Some(literal.as_str()),
            ProducerArg::Env { .. } => None,
        })
        .collect::<Option<Vec<_>>>();
    let Some(args) = literal_args else {
        return Ok(false);
    };
    match row.evidence_kind.as_str() {
        "script" => {
            let evidence_args = row.evidence_ref.split_whitespace().collect::<Vec<_>>();
            let script = evidence_args.first().copied().unwrap_or_default();
            let program = Path::new(&step.program)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(&step.program)
                .to_ascii_lowercase();
            let powershell_args = ["-NoProfile", "-File"]
                .into_iter()
                .chain(evidence_args.iter().copied())
                .collect::<Vec<_>>();
            let node_test_args = ["--test"]
                .into_iter()
                .chain(evidence_args.iter().copied())
                .collect::<Vec<_>>();
            Ok(step.program == script && args == evidence_args[1..]
                || matches!(program.as_str(), "sh" | "sh.exe" | "bash" | "bash.exe")
                    && args == evidence_args
                || matches!(
                    program.as_str(),
                    "powershell" | "powershell.exe" | "pwsh" | "pwsh.exe"
                ) && args == powershell_args
                || matches!(program.as_str(), "node" | "node.exe")
                    && (args == evidence_args || args == node_test_args))
        }
        "test" => cargo_step_executes_test(row, &args, step),
        _ => Ok(false),
    }
}

fn cargo_step_executes_test(row: &MatrixRow, args: &[&str], step: &ProducerStep) -> Result<bool> {
    if step.program != "cargo" || args.first().copied() != Some("test") {
        return Ok(false);
    }
    let selector = TestSelector::parse(&row.evidence_ref)?;
    let option_value = |option: &str| {
        args.windows(2)
            .find(|pair| pair[0] == option)
            .map(|pair| pair[1])
    };
    let package = option_value("-p").or_else(|| option_value("--package"));
    if package != Some(selector.package.as_str()) {
        return Ok(false);
    }
    if let Some(target) = selector.test_target.as_deref()
        && option_value("--test") != Some(target)
    {
        return Ok(false);
    }
    if let Some(separator) = args.iter().position(|value| *value == "--")
        && args[separator + 1..]
            .iter()
            .any(|value| !matches!(*value, "--nocapture" | "--test-threads=1"))
    {
        return Ok(false);
    }
    let separator = args
        .iter()
        .position(|value| *value == "--")
        .unwrap_or(args.len());
    let mut positionals = Vec::new();
    let mut index = 1usize;
    while index < separator {
        match args[index] {
            "-p" | "--package" | "--test" | "--bin" | "--features" | "--target" => index += 2,
            "--locked" | "--lib" | "--release" | "--all-features" | "--no-default-features" => {
                index += 1;
            }
            value if value.starts_with('-') => return Ok(false),
            value => {
                positionals.push(value);
                index += 1;
            }
        }
    }
    Ok(match selector.filter.as_deref() {
        Some(filter) => positionals.as_slice() == [filter],
        None => positionals.is_empty(),
    })
}

#[cfg(test)]
mod tests {
    use super::step_directly_executes;
    use crate::acceptance_matrix::model::MatrixRow;
    use crate::acceptance_matrix::producer::model::{ProducerArg, ProducerStep};

    fn row(kind: &str, reference: &str) -> MatrixRow {
        MatrixRow {
            requirement_id: "REQ-BINDING".into(),
            journey_id: "none".into(),
            flow_id: "none".into(),
            case_id: "BIND-001".into(),
            surface: "web".into(),
            mode: "browser".into(),
            gate: "ci".into(),
            requirement: "required".into(),
            evidence_kind: kind.into(),
            evidence_id: "test.binding".into(),
            evidence_ref: reference.into(),
            freshness: "source-bound".into(),
            note: "typed binding fixture".into(),
        }
    }

    fn step(program: &str, args: &[&str]) -> ProducerStep {
        ProducerStep {
            program: program.into(),
            args: args
                .iter()
                .map(|value| ProducerArg::Literal {
                    literal: (*value).into(),
                })
                .collect(),
        }
    }

    #[test]
    fn executable_binding_requires_the_owned_locator_to_be_invoked() {
        let test = row(
            "test",
            "cargo test -p deve_core actual_filter -- --nocapture",
        );
        assert!(
            step_directly_executes(
                &test,
                &step(
                    "cargo",
                    &[
                        "test",
                        "--locked",
                        "-p",
                        "deve_core",
                        "--lib",
                        "actual_filter"
                    ]
                )
            )
            .unwrap()
        );
        assert!(
            !step_directly_executes(
                &test,
                &step(
                    "cargo",
                    &[
                        "test",
                        "--locked",
                        "-p",
                        "deve_core",
                        "--lib",
                        "other_filter"
                    ]
                )
            )
            .unwrap()
        );
        let colliding_filter = row("test", "cargo test -p deve_core deve_core -- --nocapture");
        assert!(
            !step_directly_executes(
                &colliding_filter,
                &step("cargo", &["test", "--locked", "-p", "deve_core", "--lib"])
            )
            .unwrap()
        );
        assert!(
            !step_directly_executes(
                &test,
                &step(
                    "cargo",
                    &[
                        "test",
                        "--locked",
                        "-p",
                        "deve_core",
                        "--lib",
                        "actual_filter",
                        "--",
                        "--skip",
                        "actual_filter"
                    ]
                )
            )
            .unwrap()
        );

        let script = row("script", "scripts/check-real.sh");
        assert!(
            step_directly_executes(&script, &step("bash", &["scripts/check-real.sh"])).unwrap()
        );
        assert!(
            !step_directly_executes(&script, &step("bash", &["scripts/check-other.sh"])).unwrap()
        );
        assert!(
            !step_directly_executes(
                &script,
                &step("echo", &["incidental", "scripts/check-real.sh"])
            )
            .unwrap()
        );
        assert!(
            !step_directly_executes(&script, &step("bash", &["scripts/check-real.sh", "--help"]))
                .unwrap()
        );
    }
}
