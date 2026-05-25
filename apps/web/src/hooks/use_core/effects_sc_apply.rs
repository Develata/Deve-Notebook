//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
mod commit_refresh;
mod doc_diff;
mod fs_refresh;
mod gate;

pub(super) use commit_refresh::{CommitRefreshSignals, refresh_after_commit};
pub(super) use doc_diff::apply_doc_diff;
pub(super) use fs_refresh::{FsRefreshSignals, refresh_after_fs_change};

#[cfg(test)]
use gate::source_control_refresh_allowed;

#[cfg(test)]
mod tests;
