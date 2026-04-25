use super::context::SyncContext;
use super::history;
use super::live;
use super::scope::matches_scoped_message;
use super::snapshot;
use deve_core::models::{DocId, PeerId, RepoId};
use deve_core::protocol::ConfirmedOp;
use leptos::prelude::GetUntracked;

pub struct SnapshotDispatchMessage {
    pub repo_id: RepoId,
    pub branch: Option<PeerId>,
    pub scope_nonce: Option<u64>,
    pub doc_id: DocId,
    pub request_id: u64,
    pub content: String,
    pub base_seq: u64,
    pub version: u64,
    pub delta_ops: Vec<ConfirmedOp>,
}

pub fn handle_snapshot_message(ctx: &SyncContext, message: SnapshotDispatchMessage) {
    if !matches_scoped_message(
        super::current_scoped_message_scope(ctx),
        Some(message.repo_id),
        message.branch.clone(),
        message.scope_nonce,
    ) {
        return;
    }
    if message.doc_id != ctx.doc_id || message.request_id != ctx.open_request_id.get_untracked() {
        return;
    }
    let expected_generation = ctx.current_generation();
    snapshot::handle_snapshot(
        ctx,
        snapshot::SnapshotMessage {
            expected_generation,
            repo_id: message.repo_id,
            branch: message.branch,
            request_id: message.request_id,
            new_content: message.content,
            base_seq: message.base_seq,
            version: message.version,
            delta_ops: message.delta_ops,
        },
    );
}

pub fn handle_history_message(
    ctx: &SyncContext,
    repo_id: deve_core::models::RepoId,
    branch: Option<deve_core::models::PeerId>,
    scope_nonce: Option<u64>,
    msg_doc_id: deve_core::models::DocId,
    request_id: u64,
    ops: Vec<deve_core::protocol::ConfirmedOp>,
) {
    if !matches_scoped_message(
        super::current_scoped_message_scope(ctx),
        Some(repo_id),
        branch,
        scope_nonce,
    ) {
        return;
    }
    if msg_doc_id != ctx.doc_id || request_id != ctx.open_request_id.get_untracked() {
        return;
    }
    let expected_generation = ctx.current_generation();
    leptos::logging::log!("Received History: {} ops", ops.len());
    history::handle_history(ctx, expected_generation, ops);
}

pub fn handle_new_op_message(
    ctx: &SyncContext,
    repo_id: deve_core::models::RepoId,
    branch: Option<deve_core::models::PeerId>,
    scope_nonce: Option<u64>,
    msg_doc_id: deve_core::models::DocId,
    entry: deve_core::protocol::ConfirmedOp,
) {
    if !matches_scoped_message(
        super::current_scoped_message_scope(ctx),
        Some(repo_id),
        branch,
        scope_nonce,
    ) {
        return;
    }
    if msg_doc_id != ctx.doc_id {
        return;
    }
    live::handle_new_op(ctx, entry);
}
