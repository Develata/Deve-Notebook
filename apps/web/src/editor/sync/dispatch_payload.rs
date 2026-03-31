use super::context::SyncContext;
use super::decrypt;
use super::history_resend;
use super::key::handle_key_provide;
use super::scope::matches_scoped_message;
use deve_core::protocol::ServerError;
use deve_core::security::EncryptedOp;
use leptos::prelude::Set;

pub fn handle_write_ready_message(
    ctx: &SyncContext,
    repo_id: deve_core::models::RepoId,
    branch: Option<deve_core::models::PeerId>,
    scope_nonce: u64,
) {
    if super::accepts_current_sync_payload(ctx, repo_id, branch, scope_nonce) {
        history_resend::resend_pending_edits(ctx);
    }
}

pub fn handle_sync_push_message(
    ctx: &SyncContext,
    repo_id: deve_core::models::RepoId,
    branch: Option<deve_core::models::PeerId>,
    scope_nonce: u64,
    ops: &[EncryptedOp],
) {
    if super::accepts_current_sync_payload(ctx, repo_id, branch, scope_nonce) {
        decrypt::handle_sync_push(ctx, ops);
    }
}

pub fn handle_key_provide_message(
    ctx: &SyncContext,
    repo_id: deve_core::models::RepoId,
    branch: Option<deve_core::models::PeerId>,
    scope_nonce: u64,
    repo_key: &[u8],
) {
    if matches_scoped_message(
        super::current_scoped_message_scope(ctx),
        Some(repo_id),
        branch,
        Some(scope_nonce),
    ) {
        handle_key_provide(ctx, repo_key);
    }
}

pub fn handle_key_denied_message(
    ctx: &SyncContext,
    repo_id: Option<deve_core::models::RepoId>,
    branch: Option<deve_core::models::PeerId>,
    scope_nonce: u64,
    error: &ServerError,
) {
    if matches_scoped_message(
        super::current_scoped_message_scope(ctx),
        repo_id,
        branch,
        Some(scope_nonce),
    ) {
        ctx.set_repo_key.set(None);
        leptos::logging::warn!("KeyDenied: code={:?} detail={:?}", error.code, error.detail);
    }
}
