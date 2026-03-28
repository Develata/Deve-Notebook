use super::context::SyncContext;
use super::history;
use super::live;
use super::scope::matches_scoped_message;
use super::snapshot;
use leptos::prelude::GetUntracked;

pub fn handle_snapshot_message(
    ctx: &SyncContext,
    repo_id: deve_core::models::RepoId,
    branch: Option<deve_core::models::PeerId>,
    scope_nonce: Option<u64>,
    msg_doc_id: deve_core::models::DocId,
    request_id: u64,
    content: String,
    base_seq: u64,
    version: u64,
    delta_ops: Vec<deve_core::protocol::ConfirmedOp>,
) {
    if !matches_scoped_message(
        super::current_scoped_message_scope(ctx),
        Some(repo_id),
        branch.clone(),
        scope_nonce,
    ) {
        return;
    }
    if msg_doc_id != ctx.doc_id || request_id != ctx.open_request_id.get_untracked() {
        return;
    }
    let expected_generation = ctx.current_generation();
    snapshot::handle_snapshot(
        ctx,
        snapshot::SnapshotMessage {
            expected_generation,
            repo_id,
            branch,
            request_id,
            new_content: content,
            base_seq,
            version,
            delta_ops,
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
