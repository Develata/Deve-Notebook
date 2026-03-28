use leptos::prelude::*;

#[derive(Clone)]
pub struct SyncMergeContext {
    pub sync_mode: ReadSignal<String>,
    pub pending_ops_count: ReadSignal<u32>,
    pub pending_ops_previews: ReadSignal<Vec<(String, String, String)>>,
    pub on_get_sync_mode: Callback<()>,
    pub on_set_sync_mode: Callback<String>,
    pub on_get_pending_ops: Callback<()>,
    pub on_confirm_merge: Callback<()>,
    pub on_discard_pending: Callback<()>,
    pub on_merge_peer: Callback<String>,
}
