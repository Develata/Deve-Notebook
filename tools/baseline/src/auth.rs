//! plan_ref:
//!   - 08_auth#auth-http-endpoints
//!   - 08_auth#jwt-cookie-contract
//!   - 08_auth#password-hashing
//!   - 08_auth#cors
//!   - 08_auth#auth-rate-limiting
//!   - 08_auth#security-headers
//!   - 08_auth#audit
//!   - 08_auth#key-and-file-permissions
//!   - 08_auth#localhost-dev-policy
//!   - 08_auth#session-probe-policy
//!   - 08_auth#unauthorized-handling
//!   - 08_auth#unauthorized-disconnected-ui
//!   - 08_auth#auth-config
//!   - 23_threat_model#key-lifecycle
//!   - 23_threat_model#algorithm-deprecation
//!   - 23_threat_model#supply-chain
//!   - 23_threat_model#coordinated-vulnerability-disclosure

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
    let ctx = BaselineContext::new("auth-baseline-check")?;
    run_tsv_with_mode(&ctx, include_str!("specs/auth.tsv"), mode)?;
    ctx.ok();
    Ok(())
}
