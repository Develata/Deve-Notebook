//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 12_source_control_ui#external-changes-sibling-view
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
//! External Changes runtime facade.
//!
//! This client exposes the browser intents for projection-file changes only.
//! Source Control commit/history/graph state remains outside this facade.

use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;

#[allow(dead_code)]
#[derive(Clone)]
pub struct ExternalChangesClient {
    pub staged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub unstaged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub confirmed_changes: ReadSignal<Vec<ChangeEntry>>,
    pub on_get_changes: Callback<()>,
    pub on_stage_file: Callback<ChangeEntry>,
    pub on_stage_files: Callback<Vec<ChangeEntry>>,
    pub on_unstage_file: Callback<ChangeEntry>,
    pub on_unstage_files: Callback<Vec<ChangeEntry>>,
    pub on_discard_file: Callback<ChangeEntry>,
    pub on_apply_to_ledger: Callback<()>,
    pub on_get_doc_diff: Callback<ChangeEntry>,
}
