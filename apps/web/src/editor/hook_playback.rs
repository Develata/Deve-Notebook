//! plan_ref:
//!   - 03_rendering#large-document-runtime
//!   - 03_rendering#document-authority-bridge
//!
use super::playback;
use crate::api::WsService;
use crate::hooks::use_core::EditorContext;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_untracked};
use deve_core::models::{DocId, Op};
use leptos::prelude::*;

pub struct PlaybackEffectCtx {
    pub ws: WsService,
    pub core: EditorContext,
    pub doc_id: DocId,
    pub history: ReadSignal<Vec<(u64, Op)>>,
    pub playback_version: ReadSignal<u64>,
    pub local_version: ReadSignal<u64>,
    pub set_is_playback: WriteSignal<bool>,
}

pub fn setup_playback_effect(ctx: PlaybackEffectCtx) {
    Effect::new(move |_| {
        let version = ctx.playback_version.get();
        let local_version = ctx.local_version.get_untracked();
        playback::handle_playback_change(
            version,
            ctx.doc_id,
            local_version,
            ctx.history,
            ctx.set_is_playback,
        );
        let write_blocked = repo_write_block_untracked(
            &ctx.ws,
            RepoWriteSignals {
                load_state: ctx.core.load_state,
                is_spectator: ctx.core.is_spectator,
                handshake_ready: ctx.core.handshake_ready,
                current_repo_id: ctx.core.current_repo_id,
                current_scope_nonce: ctx.core.current_scope_nonce,
                active_branch: ctx.core.active_branch,
                pending_branch_switch: ctx.core.pending_branch_switch,
                pending_repo_switch: ctx.core.pending_repo_switch,
            },
        )
        .is_some();
        super::ffi::set_read_only(should_be_read_only(version < local_version, write_blocked));
    });
}

fn should_be_read_only(is_playback: bool, write_blocked: bool) -> bool {
    is_playback || write_blocked
}

#[cfg(test)]
mod tests {
    use super::should_be_read_only;

    #[test]
    fn playback_read_only_gate_blocks_native_runtime_write_gate() {
        assert!(should_be_read_only(false, true));
    }

    #[test]
    fn playback_read_only_gate_blocks_playback() {
        assert!(should_be_read_only(true, false));
    }

    #[test]
    fn playback_read_only_gate_allows_ready_document() {
        assert!(!should_be_read_only(false, false));
    }
}
