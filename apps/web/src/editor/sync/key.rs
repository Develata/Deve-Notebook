use super::context::SyncContext;
use super::decrypt::handle_sync_push_with_key;
use deve_core::security::RepoKey;
use leptos::prelude::Set;

/// E2EE: 收到服务端提供的 RepoKey，存入内存信号
///
/// # Invariants
/// - `repo_key` 必须恰好 32 bytes (AES-256)
/// - 仅存于 RAM 信号中，页面卸载时自动清除
pub(super) fn handle_key_provide(ctx: &SyncContext, raw: &[u8]) {
    match RepoKey::from_bytes(raw) {
        Some(key) => {
            leptos::logging::log!("E2EE: RepoKey received ({} bytes)", raw.len());
            let buffered = ctx.drain_buffered_encrypted_ops();
            ctx.set_repo_key.set(Some(key.clone()));
            if !buffered.is_empty() {
                leptos::logging::log!(
                    "E2EE: replaying {} buffered encrypted sync pushes",
                    buffered.len()
                );
                handle_sync_push_with_key(ctx, &key, &buffered);
            }
        }
        None => {
            leptos::logging::error!("E2EE: Invalid RepoKey length: {}", raw.len());
        }
    }
}
