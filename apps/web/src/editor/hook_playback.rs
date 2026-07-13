//! plan_ref:
//!   - 10_rendering#large-document-runtime
//!   - 10_rendering#document-authority-bridge
//!
use super::playback;
use deve_core::models::{DocId, Op};
use leptos::prelude::*;

pub struct PlaybackEffectCtx {
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
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn playback_effect_never_mutates_the_global_editor_readonly_adapter() {
        let source = include_str!("hook_playback.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("source before tests");

        assert!(!source.contains("set_read_only("));
        assert!(!source.contains("repo_write_block_"));
    }
}
