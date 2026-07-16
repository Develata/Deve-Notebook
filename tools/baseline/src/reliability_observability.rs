//! plan_ref:
//!   - 22_reliability_observability#slo-sli-catalog
//!   - 22_reliability_observability#telemetry-schema
//!   - 22_reliability_observability#metrics-taxonomy
//!   - 22_reliability_observability#tracing-span-boundary
//!   - 22_reliability_observability#observation-to-health-mapping
//!   - 22_reliability_observability#alerting-tier
//!   - 22_reliability_observability#resilience-playbook-index

use crate::context::BaselineContext;
use crate::spec::{RunMode, run_tsv_with_mode};
use anyhow::Result;

pub fn run() -> Result<()> {
    run_with_mode(RunMode::Full)
}

pub fn run_text() -> Result<()> {
    run_with_mode(RunMode::TextOnly)
}

fn run_with_mode(mode: RunMode) -> Result<()> {
    let ctx = BaselineContext::new("reliability-observability-baseline-check")?;
    run_tsv_with_mode(
        &ctx,
        include_str!("specs/reliability_observability.tsv"),
        mode,
    )?;
    ctx.ok();
    Ok(())
}
