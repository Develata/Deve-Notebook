//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!
//! Git ecosystem mirror bridge. Deve ledger/source-control state remains
//! authority; Git metadata is an optional external mirror.

mod executor;
mod git_cmd;
mod preflight;
mod replay;
mod status;
mod store;

pub use executor::{GitMirrorRunOptions, GitMirrorRunReport, run_pending_mirror};
pub use status::{GitMetadataKind, GitMirrorState, GitMirrorStatus, inspect_repo_root};
pub use store::{
    GIT_MIRROR_COMMITS_TABLE, GitMirrorCommitState, GitMirrorFailureStage, GitMirrorRecord,
    GitMirrorSummary, get_record, init_table, list_records, mark_committed, mark_out_of_sync,
    queue_deve_commit, summarize_records,
};
