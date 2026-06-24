//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! Browser source-control client runtime.
//!
//! This adapter emits source-control typed intents for the active scope. Core
//! and server remain the authority for staging, commit, and diff facts.

pub mod diff_session;

pub use diff_session::DiffSessionWire;

use deve_core::source_control::CommitFileDiff;
use deve_core::source_control::{ChangeEntry, CommitInfo, ConflictResolution};
use leptos::prelude::*;

#[allow(dead_code)]
#[derive(Clone)]
pub struct SourceControlClient {
    pub staged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub unstaged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub confirmed_changes: ReadSignal<Vec<ChangeEntry>>,
    pub commit_history: ReadSignal<Vec<CommitInfo>>,
    pub commit_history_request_id: ReadSignal<Option<String>>,
    pub commit_diff_request_id: ReadSignal<Option<String>>,
    pub set_commit_diff_request_id: WriteSignal<Option<String>>,
    pub on_get_changes: Callback<()>,
    pub on_stage_file: Callback<ChangeEntry>,
    pub on_stage_files: Callback<Vec<ChangeEntry>>,
    pub on_unstage_file: Callback<ChangeEntry>,
    pub on_unstage_files: Callback<Vec<ChangeEntry>>,
    pub on_discard_file: Callback<ChangeEntry>,
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
