//! plan_ref:
//!   - 03_rendering#large-document-runtime
//!   - 03_rendering#document-authority-bridge
//!
use super::playback;
use crate::api::WsService;
use crate::hooks::use_core::EditorContext;
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
        super::ffi::set_read_only(should_be_read_only(
            version < local_version,
            ctx.core.is_spectator.get_untracked(),
            ctx.core.load_state.get_untracked() != "ready",
            ctx.core.handshake_ready.get_untracked(),
            ctx.core.pending_branch_switch.get_untracked().is_some(),
            ctx.core.pending_repo_switch.get_untracked().is_some(),
            ctx.ws
                .writer_ready_for(ctx.core.current_repo_id.get_untracked().as_deref()),
        ));
    });
}

fn should_be_read_only(
    is_playback: bool,
    spectator: bool,
    loading: bool,
    handshake_ready: bool,
    branch_switching: bool,
    repo_switching: bool,
    writer_ready: bool,
) -> bool {
    is_playback
        || spectator
        || loading
        || branch_switching
        || repo_switching
        || !handshake_ready
        || !writer_ready
}
