//! plan_ref:
//!   - 07_network#full-peer-mesh-v1
//!   - 07_network#remote-shadow-apply-atomicity
//!
//! Explicitly armed Docker-smoke fault injection. Both gates are required.

use crate::server::AppState;
use anyhow::{Context, Result, anyhow};
use deve_core::models::PeerFactSeq;
use deve_core::security::EncryptedOp;
use std::sync::Arc;

pub(in crate::server) fn maybe_inject_sequence_gap(
    state: &Arc<AppState>,
    transfer_path: &'static str,
    range: Option<(PeerFactSeq, PeerFactSeq)>,
    ops: &mut Vec<EncryptedOp>,
) -> Result<()> {
    let enabled = std::env::var("DEVE_P2P_FAULT_INJECT_SEQUENCE_GAP")
        .is_ok_and(|value| matches!(value.as_str(), "1" | "true"));
    let arm_path = state
        .repo
        .ledger_dir()
        .join(".host")
        .join("test-faults")
        .join("p2p-sequence-gap-arm");
    let armed = arm_path
        .try_exists()
        .with_context(|| format!("Failed to inspect P2P sequence-gap arm file {arm_path:?}"))?;
    if !inject_sequence_gap(enabled, armed, ops) {
        return Ok(());
    }
    let (range_start, range_end) = range
        .ok_or_else(|| anyhow!("P2P sequence-gap fault requires an incremental closed range"))?;
    tracing::warn!(
        transfer_path,
        expected = %range_start,
        range_end = %range_end,
        "P2P test fault injected sequence_gap; receiver must retain its prior shadow waterline"
    );
    Ok(())
}

pub(in crate::server) fn inject_sequence_gap(
    enabled: bool,
    armed: bool,
    ops: &mut Vec<EncryptedOp>,
) -> bool {
    if !enabled || !armed || ops.is_empty() {
        return false;
    }
    ops.remove(0);
    true
}
