//! Per-producer command, environment, and evidence contract validation.
//! plan_ref: 18_release#first-tag-acceptance-matrix

use crate::acceptance_matrix::model::MatrixRow;
use crate::acceptance_matrix::producer::model::{Producer, ProducerArg, ProducerStep};
use crate::acceptance_matrix::receipt_limits::MAX_EXECUTION_RECEIPTS;
use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

const ALLOWED_TIERS: [&str; 4] = ["ci", "full", "target-host", "tag-ready"];
const ALLOWED_HOSTS: [&str; 3] = ["linux", "macos", "windows"];
const RUNNER_STATE_ENV: &str = "DEVE_ACCEPTANCE_PRODUCER_STATE_DIR";
const RUNNER_BASELINE_ENV: &str = "DEVE_BASELINE_BIN";

pub(super) fn validate_producer(
    root: &Path,
    producer: &Producer,
    matrix_evidence: &BTreeMap<&str, &MatrixRow>,
) -> Result<()> {
    if !valid_identifier(&producer.producer_id) {
        bail!(
            "acceptance producers: invalid producer_id {}",
            producer.producer_id
        );
    }
    if producer.evidence_ids.is_empty() {
        bail!(
            "acceptance producers: {} must cover at least one executable evidence",
            producer.producer_id
        );
    }
    if producer.evidence_ids.len() > MAX_EXECUTION_RECEIPTS {
        bail!(
            "acceptance producers: {} exceeds the atomic execution limit of {MAX_EXECUTION_RECEIPTS} receipts",
            producer.producer_id
        );
    }
    require_unique_nonempty(&producer.evidence_ids, "evidence_ids", producer)?;
    require_unique_nonempty(&producer.dependencies, "dependencies", producer)?;
    require_allowed_unique(&producer.tiers, &ALLOWED_TIERS, "tiers", producer)?;
    require_allowed_unique(&producer.host_os, &ALLOWED_HOSTS, "host_os", producer)?;
    if !(1..=14_400).contains(&producer.timeout_seconds) {
        bail!(
            "acceptance producers: {} timeout_seconds must be in 1..=14400",
            producer.producer_id
        );
    }
    if producer.note.trim().is_empty() {
        bail!(
            "acceptance producers: {} requires a non-empty note",
            producer.producer_id
        );
    }
    require_unique_nonempty(&producer.required_env, "required_env", producer)?;
    require_unique_nonempty(&producer.bound_env, "bound_env", producer)?;
    require_unique_nonempty(&producer.artifacts, "artifacts", producer)?;
    validate_evidence_mode(producer, matrix_evidence)?;
    validate_environment(producer)?;
    for artifact in &producer.artifacts {
        validate_artifact(root, producer, artifact)?;
    }
    if producer.steps.is_empty() {
        bail!(
            "acceptance producers: {} requires at least one step",
            producer.producer_id
        );
    }
    for step in producer.steps.iter().chain(&producer.finally_steps) {
        validate_step(root, producer, step)?;
    }
    Ok(())
}

fn validate_evidence_mode(
    producer: &Producer,
    matrix_evidence: &BTreeMap<&str, &MatrixRow>,
) -> Result<()> {
    let kinds = producer
        .evidence_ids
        .iter()
        .filter_map(|id| matrix_evidence.get(id.as_str()))
        .map(|row| row.evidence_kind.as_str())
        .collect::<BTreeSet<_>>();
    let emits_receipts = kinds.contains("receipt");
    let emits_static = kinds.iter().any(|kind| matches!(*kind, "test" | "script"));
    if emits_receipts && emits_static {
        bail!(
            "acceptance producers: {} may not mix receipt and test/script evidence",
            producer.producer_id
        );
    }
    if emits_receipts && producer.tiers.iter().any(|tier| tier == "ci") {
        bail!(
            "acceptance producers: {} receipt evidence may not use the ci tier",
            producer.producer_id
        );
    }
    if emits_static && producer.tiers.iter().any(|tier| tier != "ci") {
        bail!(
            "acceptance producers: {} test/script evidence is restricted to the ci tier",
            producer.producer_id
        );
    }
    if !emits_receipts && !producer.claims_env.is_empty() {
        bail!(
            "acceptance producers: {} test/script evidence may not declare claims_env",
            producer.producer_id
        );
    }
    Ok(())
}

fn validate_environment(producer: &Producer) -> Result<()> {
    for name in &producer.bound_env {
        if !producer.required_env.contains(name) {
            bail!(
                "acceptance producers: {} bound_env {name} must also be listed in required_env",
                producer.producer_id
            );
        }
        if sensitive_env_name(name) {
            bail!(
                "acceptance producers: {} bound_env {name} appears secret-bearing and may not be published in receipts",
                producer.producer_id
            );
        }
    }
    for name in producer
        .required_env
        .iter()
        .chain(producer.environment.keys())
        .chain(producer.claims_env.values())
    {
        if matches!(name.as_str(), RUNNER_STATE_ENV | RUNNER_BASELINE_ENV) {
            bail!(
                "acceptance producers: {} may not override runner-owned {name}",
                producer.producer_id,
            );
        }
        if !valid_env_name(name) {
            bail!(
                "acceptance producers: {} has invalid environment name {name}",
                producer.producer_id
            );
        }
    }
    for (name, value) in &producer.environment {
        if value.contains('\0') {
            bail!(
                "acceptance producers: {} environment {name} contains NUL",
                producer.producer_id
            );
        }
        if producer.claims_env.values().any(|claims| claims == name) {
            bail!(
                "acceptance producers: {} environment overrides runner-owned claims variable {name}",
                producer.producer_id
            );
        }
        if producer.required_env.contains(name) {
            bail!(
                "acceptance producers: {} environment {name} conflicts with required_env",
                producer.producer_id
            );
        }
    }
    Ok(())
}

pub(super) fn validate_step(root: &Path, producer: &Producer, step: &ProducerStep) -> Result<()> {
    if step.program.trim().is_empty()
        || step.program.contains('\0')
        || step.program.chars().any(char::is_whitespace)
    {
        bail!(
            "acceptance producers: {} has invalid step program",
            producer.producer_id
        );
    }
    for argument in &step.args {
        match argument {
            ProducerArg::LiteralString(literal) | ProducerArg::Literal { literal } => {
                if literal.contains('\0') {
                    bail!(
                        "acceptance producers: {} has a literal argument containing NUL",
                        producer.producer_id
                    );
                }
                if literal.starts_with("scripts/") && !root.join(literal).is_file() {
                    bail!(
                        "acceptance producers: {} references missing script {literal}",
                        producer.producer_id
                    );
                }
                if literal.starts_with("scripts/") && !producer.artifacts.contains(literal) {
                    bail!(
                        "acceptance producers: {} step script {literal} is missing from artifacts",
                        producer.producer_id
                    );
                }
            }
            ProducerArg::Env { env } => {
                if !valid_env_name(env) || !producer.required_env.contains(env) {
                    bail!(
                        "acceptance producers: {} argument env {env} must be listed in required_env",
                        producer.producer_id
                    );
                }
                if sensitive_env_name(env) {
                    bail!(
                        "acceptance producers: {} may not expose sensitive environment {env} through process arguments",
                        producer.producer_id
                    );
                }
            }
        }
    }
    let program_name = Path::new(&step.program)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(&step.program)
        .to_ascii_lowercase();
    validate_ci_cargo_target(producer, &program_name, &step.args)?;
    validate_shell_invocation(producer, &program_name, &step.args)?;
    Ok(())
}

fn validate_ci_cargo_target(
    producer: &Producer,
    program_name: &str,
    args: &[ProducerArg],
) -> Result<()> {
    if !producer.tiers.iter().any(|tier| tier == "ci")
        || !matches!(program_name, "cargo" | "cargo.exe")
    {
        return Ok(());
    }
    let literals = args.iter().filter_map(|argument| match argument {
        ProducerArg::LiteralString(literal) | ProducerArg::Literal { literal } => {
            Some(literal.as_str())
        }
        ProducerArg::Env { .. } => None,
    });
    let mut saw_test = false;
    let mut saw_target = false;
    for literal in literals {
        saw_test |= literal == "test";
        saw_target |= matches!(literal, "--lib" | "--bin" | "--test");
    }
    if saw_test && !saw_target {
        bail!(
            "acceptance producers: {} CI cargo test step requires --lib, --bin, or --test",
            producer.producer_id
        );
    }
    Ok(())
}

fn validate_artifact(root: &Path, producer: &Producer, artifact: &str) -> Result<()> {
    let path = Path::new(artifact);
    if !artifact.starts_with("scripts/")
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || !root.join(path).is_file()
    {
        bail!(
            "acceptance producers: {} has invalid or missing artifact {artifact}",
            producer.producer_id
        );
    }
    Ok(())
}

pub(super) fn validate_shell_invocation(
    producer: &Producer,
    program_name: &str,
    args: &[ProducerArg],
) -> Result<()> {
    let literal = |index: usize| match args.get(index) {
        Some(ProducerArg::LiteralString(literal) | ProducerArg::Literal { literal }) => {
            Some(literal.as_str())
        }
        _ => None,
    };
    match program_name {
        "sh" | "sh.exe" | "bash" | "bash.exe" => {
            if literal(0).is_none_or(|value| {
                value.starts_with('-') || !value.starts_with("scripts/") || !value.ends_with(".sh")
            }) {
                bail!(
                    "acceptance producers: {} shell must execute a registered .sh file directly",
                    producer.producer_id
                );
            }
        }
        "powershell" | "powershell.exe" | "pwsh" | "pwsh.exe" => {
            if literal(0).is_none_or(|value| !value.eq_ignore_ascii_case("-NoProfile"))
                || literal(1).is_none_or(|value| !value.eq_ignore_ascii_case("-File"))
                || literal(2)
                    .is_none_or(|value| !value.starts_with("scripts/") || !value.ends_with(".ps1"))
            {
                bail!(
                    "acceptance producers: {} PowerShell must use -NoProfile -File <registered.ps1>",
                    producer.producer_id
                );
            }
        }
        "cmd" | "cmd.exe" => {
            bail!(
                "acceptance producers: {} may not use cmd command-string execution",
                producer.producer_id
            );
        }
        _ => {}
    }
    Ok(())
}

pub(in crate::acceptance_matrix::producer) fn contract_fingerprint(
    producer: &Producer,
) -> Result<String> {
    let bytes = serde_json::to_vec(producer).context("failed to serialize producer contract")?;
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Ok(format!("fnv1a64:{hash:016x}"))
}

fn require_unique_nonempty(values: &[String], field: &str, producer: &Producer) -> Result<()> {
    if values.iter().any(|value| value.trim().is_empty()) {
        bail!(
            "acceptance producers: {} has an empty {field} value",
            producer.producer_id
        );
    }
    let unique: BTreeSet<_> = values.iter().collect();
    if unique.len() != values.len() {
        bail!(
            "acceptance producers: {} has duplicate {field} values",
            producer.producer_id
        );
    }
    Ok(())
}

fn require_allowed_unique(
    values: &[String],
    allowed: &[&str],
    field: &str,
    producer: &Producer,
) -> Result<()> {
    require_unique_nonempty(values, field, producer)?;
    if values
        .iter()
        .any(|value| !allowed.contains(&value.as_str()))
    {
        bail!(
            "acceptance producers: {} has unsupported {field}",
            producer.producer_id
        );
    }
    Ok(())
}

pub(super) fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
}

pub(super) fn valid_env_name(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte == b'_' || byte.is_ascii_uppercase())
        && bytes.all(|byte| byte == b'_' || byte.is_ascii_uppercase() || byte.is_ascii_digit())
}

pub(super) fn sensitive_env_name(value: &str) -> bool {
    value.split('_').any(|segment| {
        matches!(
            segment,
            "SECRET" | "PASSWORD" | "TOKEN" | "PRIVATE" | "CREDENTIAL" | "CREDENTIALS" | "KEY"
        )
    })
}
