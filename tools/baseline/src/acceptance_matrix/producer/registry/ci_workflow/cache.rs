//! Source-only Cargo cache contract for check-only required jobs.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::command::{reject_step_execution_modifiers, reject_tolerated_or_conditional};
use super::yaml::{as_mapping, as_string, optional, required};
use crate::release::toolchain::{EXACT_TOOLCHAIN, RUST_ACTION_REF};
use anyhow::{Result, bail};
use std::collections::BTreeSet;
use yaml_rust2::Yaml;

const CACHE_ACTION: &str = "actions/cache@v6";
const ALLOWED_ACTIONS: [&str; 4] = [
    "actions/checkout@v6",
    RUST_ACTION_REF,
    "actions/setup-node@v6",
    CACHE_ACTION,
];
const CACHE_PATHS: [&str; 4] = [
    "~/.cargo/registry/index",
    "~/.cargo/registry/cache",
    "~/.cargo/registry/src",
    "~/.cargo/git/db",
];

pub(super) fn validate_required_job_cache(steps: &[Yaml], job_path: &str) -> Result<()> {
    let mut cache_steps = Vec::new();
    let first_run = steps.iter().position(|value| {
        value
            .as_hash()
            .is_some_and(|step| optional(step, "run").is_some())
    });
    for (index, value) in steps.iter().enumerate() {
        let step_path = format!("{job_path}.steps[{index}]");
        let step = as_mapping(value, &step_path)?;
        if let Some(action) = optional(step, "uses").and_then(Yaml::as_str) {
            if !ALLOWED_ACTIONS.contains(&action) {
                bail!(
                    "acceptance producers: {step_path} uses unsupported action {action}; required jobs forbid alternate build/cache actions"
                );
            }
            if action == CACHE_ACTION {
                cache_steps.push((index, step, step_path));
            }
        }
    }
    if cache_steps.len() != 1 {
        bail!(
            "acceptance producers: {job_path} must contain exactly one source-only Cargo cache step"
        );
    }
    let (cache_index, step, step_path) = cache_steps.pop().expect("one cache step");
    reject_tolerated_or_conditional(step, &step_path)?;
    reject_step_execution_modifiers(step, &step_path)?;
    if as_string(
        required(step, "uses", &step_path)?,
        &format!("{step_path}.uses"),
    )? != CACHE_ACTION
    {
        bail!("acceptance producers: {step_path} must use exact {CACHE_ACTION}");
    }
    let first_run = first_run.ok_or_else(|| {
        anyhow::anyhow!("acceptance producers: {job_path} has no command after its source cache")
    })?;
    if cache_index > first_run {
        bail!("acceptance producers: {step_path} must precede every command step");
    }
    let first_run_path = format!("{job_path}.steps[{first_run}].run");
    let first_run_step = as_mapping(&steps[first_run], &format!("{job_path}.steps[{first_run}]"))?;
    if as_string(
        required(first_run_step, "run", &first_run_path)?,
        &first_run_path,
    )?
    .trim()
        != "cargo fetch --locked"
    {
        bail!(
            "acceptance producers: {job_path} first command must be exact `cargo fetch --locked` so every cache writer has the complete locked source set"
        );
    }

    let with_path = format!("{step_path}.with");
    let with = as_mapping(required(step, "with", &step_path)?, &with_path)?;
    if with.len() != 3 {
        bail!(
            "acceptance producers: {with_path} must contain exact path, key, and restore-keys fields"
        );
    }
    let path_text = as_string(
        required(with, "path", &with_path)?,
        &format!("{with_path}.path"),
    )?;
    let paths = path_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let actual_paths = paths.iter().copied().collect::<BTreeSet<_>>();
    let expected_paths = CACHE_PATHS.into_iter().collect::<BTreeSet<_>>();
    if paths.len() != actual_paths.len() || actual_paths != expected_paths {
        bail!(
            "acceptance producers: {step_path} source-only Cargo cache paths drifted; expected={expected_paths:?} actual={actual_paths:?}"
        );
    }
    let cache_key = format!(
        "${{{{ runner.os }}}}-cargo-source-rust-{EXACT_TOOLCHAIN}-${{{{ hashFiles('Cargo.lock') }}}}"
    );
    if as_string(
        required(with, "key", &with_path)?,
        &format!("{with_path}.key"),
    )? != cache_key
    {
        bail!(
            "acceptance producers: {step_path} must use exact source-only Cargo cache key {cache_key}"
        );
    }
    let cache_restore_key = format!("${{{{ runner.os }}}}-cargo-source-rust-{EXACT_TOOLCHAIN}-");
    if as_string(
        required(with, "restore-keys", &with_path)?,
        &format!("{with_path}.restore-keys"),
    )?
    .trim()
        != cache_restore_key
    {
        bail!(
            "acceptance producers: {step_path} must use exact source-only Cargo restore prefix {cache_restore_key}"
        );
    }
    Ok(())
}
