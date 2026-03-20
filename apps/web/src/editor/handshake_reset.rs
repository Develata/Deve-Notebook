use super::buffered_ops::clear_sync_buffers;
use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::EditorContext;
use deve_core::protocol::ConfirmedOp;
use deve_core::security::{EncryptedOp, RepoKey};
use leptos::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct HandshakeResetCtx {
    pub ws: WsService,
    pub core: EditorContext,
    pub ready_generation: Arc<AtomicU64>,
    pub buffered_live_ops: Arc<Mutex<Vec<ConfirmedOp>>>,
    pub buffered_encrypted_ops: Arc<Mutex<Vec<EncryptedOp>>>,
    pub set_repo_key: WriteSignal<Option<RepoKey>>,
}

pub fn setup_handshake_reset_effect(ctx: HandshakeResetCtx) {
    Effect::new(move |_| {
        if !should_reset_sync_buffers(
            ctx.ws.status.get() == ConnectionStatus::Connected,
            ctx.core.handshake_ready.get(),
            ctx.core.pending_branch_switch.get().is_some(),
            ctx.core.pending_repo_switch.get().is_some(),
        ) {
            return;
        }
        ctx.ready_generation.store(0, Ordering::Relaxed);
        clear_sync_buffers(
            &ctx.buffered_live_ops,
            &ctx.buffered_encrypted_ops,
            "握手重置时忽略 buffered live ops",
            "握手重置时忽略 buffered encrypted ops",
        );
        ctx.set_repo_key.set(None);
    });
}

fn should_reset_sync_buffers(
    connected: bool,
    handshake_ready: bool,
    pending_branch_switch: bool,
    pending_repo_switch: bool,
) -> bool {
    !connected || !handshake_ready || pending_branch_switch || pending_repo_switch
}

#[cfg(test)]
mod tests {
    use super::should_reset_sync_buffers;

    #[test]
    fn resets_buffers_when_connection_or_handshake_is_not_ready() {
        assert!(should_reset_sync_buffers(false, true, false, false));
        assert!(should_reset_sync_buffers(true, false, false, false));
    }

    #[test]
    fn resets_buffers_while_scope_switch_is_pending() {
        assert!(should_reset_sync_buffers(true, true, true, false));
        assert!(should_reset_sync_buffers(true, true, false, true));
    }

    #[test]
    fn keeps_buffers_when_scope_is_stable_and_ready() {
        assert!(!should_reset_sync_buffers(true, true, false, false));
    }
}
