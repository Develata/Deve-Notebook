//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::hooks::use_core::write_gate::{RepoWriteGateState, repo_source_control_read_block};

pub(super) fn source_control_refresh_allowed(
    expected_scope_nonce: u64,
    current_scope_nonce: u64,
    gate_state: RepoWriteGateState<'_>,
) -> bool {
    expected_scope_nonce == current_scope_nonce
        && repo_source_control_read_block(gate_state).is_none()
}
