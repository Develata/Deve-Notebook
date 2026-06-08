//! plan_ref:
//!   - 10_rendering#large-document-runtime
//!   - 10_rendering#document-authority-bridge
//!
use super::super::contexts::EditorContext;
use super::super::types::CoreState;

pub(super) fn build_editor_context(state: &CoreState) -> EditorContext {
    let document = &state.runtime_clients.document;
    let rendering = &state.runtime_clients.rendering;
    let scope = &state.runtime_clients.scope;
    let session = &state.runtime_clients.session;
    EditorContext {
        docs: document.docs,
        current_doc: document.current_doc,
        stats: rendering.stats,
        on_stats: rendering.on_stats,
        load_state: rendering.load_state,
        set_load_state: rendering.set_load_state,
        load_progress: rendering.load_progress,
        set_load_progress: rendering.set_load_progress,
        load_eta_ms: rendering.load_eta_ms,
        set_load_eta_ms: rendering.set_load_eta_ms,
        doc_version: state.doc_version,
        set_doc_version: state.set_doc_version,
        playback_version: state.playback_version,
        set_playback_version: state.set_playback_version,
        is_spectator: scope.is_spectator,
        active_branch: scope.active_branch,
        pending_branch_switch: state.pending_branch_switch,
        current_repo_id: scope.current_repo_id,
        current_scope_nonce: scope.current_scope_nonce,
        pending_repo_switch: scope.pending_repo_switch,
        handshake_ready: session.handshake_ready,
        handshake_scope_nonce: session.handshake_scope_nonce,
        pending_local_edits: document.pending_local_edits,
        set_pending_local_edits: document.set_pending_local_edits,
        set_pending_navigation: state.set_pending_navigation,
    }
}
