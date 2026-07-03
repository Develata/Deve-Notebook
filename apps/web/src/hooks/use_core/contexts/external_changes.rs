//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 12_source_control_ui#external-changes-sibling-view
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use leptos::prelude::*;

use super::super::source_control_notice::SourceControlNotice;
use super::super::types::{PendingBranchSwitch, PendingRepoSwitch};
use super::super::write_gate::RepoWriteBlock;
use deve_core::models::PeerId;
use deve_core::source_control::ChangeEntry;

#[derive(Clone)]
pub struct ExternalChangesContext {
    pub staged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub unstaged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub confirmed_changes: ReadSignal<Vec<ChangeEntry>>,
    pub can_write: Signal<bool>,
    pub write_block: Signal<Option<RepoWriteBlock>>,
    pub read_block: Signal<Option<RepoWriteBlock>>,
    pub notice: ReadSignal<Option<SourceControlNotice>>,
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
    pub on_apply_to_ledger: Callback<()>,
    pub on_get_doc_diff: Callback<ChangeEntry>,
}
