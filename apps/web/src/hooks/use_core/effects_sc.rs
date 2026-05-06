//! Source Control 消息分发。
//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime

#[path = "effects_sc_context.rs"]
mod context;
#[path = "effects_sc_dispatch.rs"]
mod dispatch;
#[path = "effects_sc_dispatch_ack_helpers.rs"]
mod dispatch_ack_helpers;
#[path = "effects_sc_dispatch_acks.rs"]
mod dispatch_acks;
#[path = "effects_sc_dispatch_lists.rs"]
mod dispatch_lists;

pub(crate) use context::ScMessageContext;
pub(crate) use dispatch::handle_sc_message;

#[cfg(test)]
#[path = "effects_sc_test.rs"]
mod tests;
