//! Canonical command and toolchain policy for CI producer jobs.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::yaml::{as_mapping, as_sequence, as_string, optional, required, scalar_text};
use crate::release::toolchain::{EXACT_TOOLCHAIN, RUST_ACTION_REF};
use anyhow::{Context, Result, bail};
use std::collections::BTreeSet;
use yaml_rust2::Yaml;
use yaml_rust2::yaml::Hash;

pub(super) struct JobCommands {
    pub(super) plans: Vec<BTreeSet<String>>,
    pub(super) executions: Vec<BTreeSet<String>>,
    pub(super) first_command_step: Option<usize>,
}

pub(super) fn job_commands(job: &Hash, job_path: &str) -> Result<JobCommands> {
    let Some(raw_steps) = optional(job, "steps") else {
        return Ok(JobCommands {
            plans: Vec::new(),
            executions: Vec::new(),
            first_command_step: None,
        });
    };
    let steps = as_sequence(raw_steps, &format!("{job_path}.steps"))?;
    let mut plans = Vec::new();
    let mut executions = Vec::new();
    let mut first_command_step = None;
    for (index, value) in steps.iter().enumerate() {
        let step_path = format!("{job_path}.steps[{index}]");
        let step = as_mapping(value, &step_path)?;
        let Some(run) = optional(step, "run") else {
            continue;
        };
        let run = as_string(run, &format!("{step_path}.run"))?;
        for command in parse_run(run, &format!("{step_path}.run"))? {
            first_command_step.get_or_insert(index);
            reject_tolerated_or_conditional(step, &step_path)?;
            reject_step_execution_modifiers(step, &step_path)?;
            if command.plan {
                plans.push(command.producers);
            } else {
                executions.push(command.producers);
            }
        }
    }
    Ok(JobCommands {
        plans,
        executions,
        first_command_step,
    })
}

struct CiCommand {
    plan: bool,
    producers: BTreeSet<String>,
}

fn parse_run(run: &str, path: &str) -> Result<Vec<CiCommand>> {
    if !run.contains("acceptance-run") && !run.contains("--tier ci") {
        return Ok(Vec::new());
    }
    let mut commands = Vec::new();
    for line in run.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if !line.contains("acceptance-run") {
            bail!(
                "acceptance producers: {path} mixes an acceptance command with inert or compound shell text"
            );
        }
        if let Some(command) = parse_command(line, path)? {
            commands.push(command);
        }
    }
    Ok(commands)
}

fn parse_command(command: &str, path: &str) -> Result<Option<CiCommand>> {
    let tokens = command.split_whitespace().collect::<Vec<_>>();
    let mut index = 0usize;
    for expected in ["cargo", "run"] {
        expect_token(&tokens, &mut index, expected, path)?;
    }
    expect_token(&tokens, &mut index, "--locked", path)?;
    if tokens.get(index) == Some(&"--quiet") {
        index += 1;
    }
    for expected in ["-p", "deve_baseline", "--", "acceptance-run", "--tier"] {
        expect_token(&tokens, &mut index, expected, path)?;
    }
    let tier = tokens
        .get(index)
        .with_context(|| format!("acceptance producers: {path} is missing the acceptance tier"))?;
    index += 1;
    let plan = tokens.get(index) == Some(&"--plan");
    if plan {
        index += 1;
    }
    let mut producers = BTreeSet::new();
    while index < tokens.len() {
        expect_token(&tokens, &mut index, "--producer", path)?;
        let producer = tokens.get(index).with_context(|| {
            format!("acceptance producers: {path} has --producer without an ID")
        })?;
        if !producer.starts_with("ci.")
            || !producer
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-'))
        {
            bail!("acceptance producers: {path} has invalid CI producer ID {producer}");
        }
        if !producers.insert((*producer).to_owned()) {
            bail!("acceptance producers: {path} repeats producer {producer}");
        }
        index += 1;
    }
    if *tier != "ci" {
        if !plan || !producers.is_empty() {
            bail!("acceptance producers: {path} may only plan non-CI tiers in check.yml");
        }
        return Ok(None);
    }
    if !plan && producers.is_empty() {
        bail!(
            "acceptance producers: {path} executes the multi-host CI tier without explicit --producer filters"
        );
    }
    Ok(Some(CiCommand { plan, producers }))
}

fn expect_token(tokens: &[&str], index: &mut usize, expected: &str, path: &str) -> Result<()> {
    if tokens.get(*index) != Some(&expected) {
        bail!(
            "acceptance producers: {path} must use the canonical acceptance argv; expected {expected} at token {}",
            *index
        );
    }
    *index += 1;
    Ok(())
}

pub(super) fn reject_tolerated_or_conditional(mapping: &Hash, path: &str) -> Result<()> {
    for key in ["if", "continue-on-error"] {
        if optional(mapping, key).is_some() {
            bail!(
                "acceptance producers: {path} may not declare {key}; required producer execution cannot be skipped or tolerated"
            );
        }
    }
    Ok(())
}

pub(super) fn reject_step_execution_modifiers(mapping: &Hash, path: &str) -> Result<()> {
    for key in ["shell", "timeout-minutes", "env", "working-directory"] {
        if optional(mapping, key).is_some() {
            bail!(
                "acceptance producers: {path} may not declare {key}; canonical producer execution may not be reinterpreted"
            );
        }
    }
    Ok(())
}

pub(super) fn reject_job_execution_modifiers(mapping: &Hash, path: &str) -> Result<()> {
    for key in ["strategy", "defaults", "env", "container", "services"] {
        if optional(mapping, key).is_some() {
            bail!(
                "acceptance producers: {path} may not declare {key}; producer jobs must preserve one canonical host execution"
            );
        }
    }
    Ok(())
}

pub(super) fn require_rust_toolchain(
    steps: &[Yaml],
    before_step: usize,
    job_path: &str,
) -> Result<()> {
    if has_setup_action(
        steps,
        before_step,
        RUST_ACTION_REF,
        "toolchain",
        EXACT_TOOLCHAIN,
    )? {
        Ok(())
    } else {
        bail!(
            "acceptance producers: {job_path} must install exact Rust {EXACT_TOOLCHAIN} before producer commands"
        )
    }
}

pub(super) fn require_node_24(steps: &[Yaml], before_step: usize, job_path: &str) -> Result<()> {
    if has_setup_action(
        steps,
        before_step,
        "actions/setup-node@v6",
        "node-version",
        "24",
    )? {
        Ok(())
    } else {
        bail!("acceptance producers: {job_path} must install exact Node.js 24")
    }
}

fn has_setup_action(
    steps: &[Yaml],
    before_step: usize,
    action_ref: &str,
    input: &str,
    expected: &str,
) -> Result<bool> {
    for (index, value) in steps.iter().take(before_step).enumerate() {
        let path = format!("check.yml setup step[{index}]");
        let step = as_mapping(value, &path)?;
        let Some(uses) = optional(step, "uses") else {
            continue;
        };
        if as_string(uses, &format!("{path}.uses"))? != action_ref {
            continue;
        }
        reject_tolerated_or_conditional(step, &path)?;
        reject_step_execution_modifiers(step, &path)?;
        let with = as_mapping(required(step, "with", &path)?, &format!("{path}.with"))?;
        return Ok(scalar_text(required(with, input, &path)?)? == expected);
    }
    Ok(false)
}
