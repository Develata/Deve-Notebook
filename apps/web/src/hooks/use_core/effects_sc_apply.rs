//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
#[path = "effects_sc_apply_commit.rs"]
mod commit_refresh;
#[path = "effects_sc_apply_doc_diff.rs"]
mod doc_diff;
#[path = "effects_sc_apply_fs.rs"]
mod fs_refresh;
#[path = "effects_sc_apply_gate.rs"]
mod gate;

pub(super) use commit_refresh::{CommitRefreshSignals, refresh_after_commit};
pub(super) use doc_diff::apply_doc_diff;
pub(super) use fs_refresh::{FsRefreshSignals, refresh_after_fs_change};

#[cfg(test)]
use gate::source_control_refresh_allowed;

#[cfg(test)]
#[path = "effects_sc_apply_test.rs"]
mod tests;
