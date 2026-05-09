//! plan_ref:
//!   - 04_storage#git-ecosystem-coexistence
//!   - 07_diff_logic#git-mirror-lifecycle
//!
//! Git ecosystem mirror bridge. Deve ledger/source-control state remains
//! authority; Git metadata is an optional external mirror.

mod error;
mod executor;
mod failure_metadata;
mod git_cmd;
mod import_apply;
mod import_plan;
mod preflight;
mod push;
mod repair_action;
mod repair_review;
mod replay;
mod replay_git;
mod replay_plan;
mod replay_snapshot;
mod status;
mod store;

pub use error::{
    GitImportApplyError, GitImportApplyResult, GitImportPlanError, GitImportPlanResult,
    GitMirrorPushError, GitMirrorPushResult, GitMirrorRunError, GitMirrorRunResult,
    GitMirrorStatusError, GitMirrorStatusResult, GitMirrorStoreError, GitMirrorStoreResult,
};
pub use executor::{GitMirrorRunOptions, GitMirrorRunReport, export_mirror, run_pending_mirror};
pub use import_apply::{GitImportApplyReport, apply_import};
pub use import_plan::{GitImportPlan, GitImportPlanBlocker, GitImportPlanEntry, plan_import};
pub use push::{GitMirrorPushBlocker, GitMirrorPushOptions, GitMirrorPushReport, push_mirror};
pub use repair_action::{GitMirrorRepairAction, GitMirrorRepairActionCode};
pub use repair_review::{GitMirrorRepairReview, GitMirrorRepairReviewRecord, build_repair_review};
pub use status::{GitMetadataKind, GitMirrorState, GitMirrorStatus, inspect_repo_root};
pub use store::{
    GIT_MIRROR_COMMITS_TABLE, GitMirrorCommitState, GitMirrorFailureStage, GitMirrorRecord,
    GitMirrorSummary, get_record, init_table, list_records, mark_committed, mark_out_of_sync,
    queue_deve_commit, summarize_records,
};
