//! Candidate producer job execution defaults and outer deadline validation.
//! plan_ref:
//!   - 18_release#first-tag-acceptance-matrix

use super::{mapping, optional, required, string};
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;

const PRODUCER_JOB_MARGIN_SECONDS: u64 = 15 * 60;

pub(super) fn validate_budget(
    job_path: &str,
    job: &yaml_rust2::yaml::Hash,
    producers: &[String],
    producer_timeouts: &BTreeMap<&str, u64>,
) -> Result<()> {
    let timeout_minutes = required(job, "timeout-minutes", job_path)?
        .as_i64()
        .filter(|value| *value > 0)
        .map(|value| value as u64)
        .with_context(|| {
            format!("acceptance producers: {job_path}.timeout-minutes must be a positive integer")
        })?;
    let producer_seconds = producers.iter().try_fold(0u64, |total, producer| {
        let timeout = producer_timeouts
            .get(producer.as_str())
            .with_context(|| format!("acceptance producers: unknown producer {producer}"))?;
        total
            .checked_add(*timeout)
            .context("acceptance producers: producer timeout sum overflow")
    })?;
    let required_seconds = producer_seconds
        .checked_add(PRODUCER_JOB_MARGIN_SECONDS)
        .context("acceptance producers: producer job budget overflow")?;
    if timeout_minutes.saturating_mul(60) < required_seconds {
        bail!(
            "acceptance producers: {job_path} timeout {timeout_minutes}m is below producer deadlines plus {}m margin",
            PRODUCER_JOB_MARGIN_SECONDS / 60
        );
    }
    Ok(())
}

pub(super) fn validate_defaults(job: &yaml_rust2::yaml::Hash, job_path: &str) -> Result<()> {
    let Some(defaults) = optional(job, "defaults") else {
        return Ok(());
    };
    let defaults = mapping(defaults, &format!("{job_path}.defaults"))?;
    if defaults.len() != 1 {
        bail!("acceptance producers: {job_path}.defaults must contain only run.shell");
    }
    let run = mapping(
        required(defaults, "run", &format!("{job_path}.defaults"))?,
        &format!("{job_path}.defaults.run"),
    )?;
    if run.len() != 1
        || string(
            required(run, "shell", &format!("{job_path}.defaults.run"))?,
            &format!("{job_path}.defaults.run.shell"),
        )? != "bash"
    {
        bail!("acceptance producers: {job_path}.defaults.run.shell must be exact bash");
    }
    Ok(())
}
