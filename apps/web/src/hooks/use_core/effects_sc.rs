//! Source Control 消息分发。
//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime

mod context;
mod dispatch;
mod dispatch_ack_helpers;
mod dispatch_acks;
mod dispatch_lists;

pub(crate) use context::ScMessageContext;
pub(crate) use dispatch::handle_sc_message;

#[cfg(test)]
mod tests;
