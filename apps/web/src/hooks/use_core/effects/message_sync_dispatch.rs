use crate::api::WsService;
use deve_core::protocol::ServerMessage;

use super::super::effects_sc;
use super::super::state::CoreSignals;
use super::message_runtime_remaining::handle_remaining;

pub fn handle_sc_or_remaining<F>(
    msg: ServerMessage,
    ws: &WsService,
    signals: CoreSignals,
    schedule_refresh: &F,
) where
    F: Fn(),
{
    let ctx = effects_sc::sc_message_context(
        signals.set_staged_changes,
        signals.set_unstaged_changes,
        signals.changes_request_id,
        signals.set_changes_request_id,
        signals.set_commit_history,
        signals.commit_history_request_id,
        signals.set_commit_history_request_id,
        signals.set_doc_list_request_id,
        signals.set_tree_request_id,
        signals.doc_diff_request_id,
        signals.set_doc_diff_request_id,
        signals.set_diff_content,
        signals.commit_diff_request_id,
        signals.set_commit_diff_request_id,
        signals.set_commit_diff_result,
        signals.current_repo_id,
        signals.active_branch,
        signals.pending_branch_switch,
        signals.pending_repo_switch,
        signals.current_scope_nonce,
        schedule_refresh,
        ws,
    );
    if !effects_sc::handle_sc_message(&msg, &ctx) {
        handle_remaining(msg, signals);
    }
}
