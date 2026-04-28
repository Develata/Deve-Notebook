//! plan_ref:
//!   - 03_rendering#large-document-runtime
//!   - 03_rendering#document-authority-bridge
//!
use super::super::contexts::EditorContext;
use super::super::types::CoreState;

pub(super) fn build_editor_context(state: &CoreState) -> EditorContext {
    EditorContext {
        docs: state.docs,
        current_doc: state.current_doc,
        stats: state.stats,
        on_stats: state.on_stats,
        load_state: state.load_state,
        set_load_state: state.set_load_state,
        load_progress: state.load_progress,
        set_load_progress: state.set_load_progress,
        load_eta_ms: state.load_eta_ms,
        set_load_eta_ms: state.set_load_eta_ms,
        doc_version: state.doc_version,
        set_doc_version: state.set_doc_version,
        playback_version: state.playback_version,
        set_playback_version: state.set_playback_version,
        is_spectator: state.is_spectator,
        active_branch: state.active_branch,
        pending_branch_switch: state.pending_branch_switch,
        current_repo_id: state.current_repo_id,
        current_scope_nonce: state.current_scope_nonce,
        pending_repo_switch: state.pending_repo_switch,
        handshake_ready: state.handshake_ready,
        handshake_scope_nonce: state.handshake_scope_nonce,
        pending_local_edits: state.pending_local_edits,
        set_pending_local_edits: state.set_pending_local_edits,
    }
}
