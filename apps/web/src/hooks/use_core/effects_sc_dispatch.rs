use super::ScMessageContext;
use super::dispatch_acks::handle_sc_ack_message;
use super::dispatch_lists::handle_sc_list_message;
use deve_core::protocol::ServerMessage;

pub(crate) fn handle_sc_message(msg: &ServerMessage, ctx: &ScMessageContext<'_>) -> bool {
    let active_scope_nonce = ctx.active_scope_nonce();
    if handle_sc_list_message(msg, ctx, active_scope_nonce) {
        return true;
    }
    if handle_sc_ack_message(msg, ctx, active_scope_nonce) {
        return true;
    }
    false
}
