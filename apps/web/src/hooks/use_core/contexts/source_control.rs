//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use leptos::prelude::*;

use super::super::diff_session::DiffSessionWire;
use super::super::source_control_notice::SourceControlNotice;
use super::super::types::{PendingBranchSwitch, PendingRepoSwitch};
use super::super::write_gate::RepoWriteBlock;
use deve_core::models::PeerId;
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo, ConflictResolution};

#[derive(Clone)]
pub struct SourceControlContext {
    pub staged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub unstaged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub confirmed_changes: ReadSignal<Vec<ChangeEntry>>,
    pub commit_history: ReadSignal<Vec<CommitInfo>>,
    pub commit_history_request_id: ReadSignal<Option<String>>,
    pub commit_diff_request_id: ReadSignal<Option<String>>,
    pub set_commit_diff_request_id: WriteSignal<Option<String>>,
    pub can_write: Signal<bool>,
    pub write_block: Signal<Option<RepoWriteBlock>>,
    pub read_block: Signal<Option<RepoWriteBlock>>,
    pub git_bridge_mode: ReadSignal<String>,
    pub notice: ReadSignal<Option<SourceControlNotice>>,
    pub set_notice: WriteSignal<Option<SourceControlNotice>>,
    pub clear_notice: Callback<()>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub current_scope_nonce: ReadSignal<u64>,
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchSwitch>>,
    pub pending_repo_switch: ReadSignal<Option<PendingRepoSwitch>>,
    pub on_get_changes: Callback<()>,
    pub on_stage_file: Callback<ChangeEntry>,
    pub on_stage_files: Callback<Vec<ChangeEntry>>,
    pub on_unstage_file: Callback<ChangeEntry>,
    pub on_unstage_files: Callback<Vec<ChangeEntry>>,
    pub on_discard_file: Callback<ChangeEntry>,
    pub on_discard_pending: Callback<()>,
    pub on_commit: Callback<String>,
    pub on_get_history: Callback<u32>,
    pub diff_content: ReadSignal<Option<DiffSessionWire>>,
    pub set_diff_content: WriteSignal<Option<DiffSessionWire>>,
    pub on_get_doc_diff: Callback<ChangeEntry>,
    pub commit_diff_result: ReadSignal<Vec<CommitFileDiff>>,
    pub set_commit_diff_result: WriteSignal<Vec<CommitFileDiff>>,
    pub on_resolve_conflict: Callback<(ChangeEntry, ConflictResolution)>,
    pub on_get_commit_diff: Callback<(Option<String>, String)>,
    pub on_commit_and_push: Callback<String>,
}
