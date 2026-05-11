use super::super::contexts::SyncMergeContext;
use super::super::types::CoreState;

pub(super) fn build_sync_context(state: &CoreState) -> SyncMergeContext {
    SyncMergeContext {
        sync_mode: state.sync_mode,
        pending_ops_count: state.pending_ops_count,
        pending_ops_previews: state.pending_ops_previews,
        on_get_sync_mode: state.on_get_sync_mode,
        on_set_sync_mode: state.on_set_sync_mode,
        on_get_pending_ops: state.on_get_pending_ops,
        on_confirm_merge: state.on_confirm_merge,
        on_discard_pending: state.on_discard_pending,
        on_merge_peer: state.on_merge_peer,
    }
}
