//! plan_ref:
//!   - 07_network#full-peer-mesh-v1
//!   - 07_network#full-peer-ws-admission
//!   - 07_network#repo-scoped-handshake
//!   - 07_network#remote-shadow-apply-atomicity
//!   - 07_network#relay-proxy-attribution-contract
//!   - 07_network#projection-recovery-contract
//!   - 07_network#server-ws-runtime
//!   - 07_network#web-ws-runtime

use crate::context::BaselineContext;
use crate::spec::run_tsv;
use anyhow::Result;

pub fn run() -> Result<()> {
    let ctx = BaselineContext::new("network-baseline-check")?;
    run_tsv(&ctx, include_str!("specs/network.tsv"))?;
    ctx.ok();
    Ok(())
}
