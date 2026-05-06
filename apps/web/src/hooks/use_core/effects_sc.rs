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

#[allow(unused_imports)]
pub(crate) use super::effects_sc_scope::{matches_current_repo, matches_current_scope};
pub(crate) use super::effects_sc_state::clear_repo_scoped_state;
#[cfg(test)]
pub(crate) use super::effects_sc_state::{
    changes_list_matches_request, commit_diff_matches_request, commit_history_matches_request,
    doc_diff_matches_request, scoped_ack_matches,
};

#[cfg(test)]
#[path = "effects_sc_test.rs"]
mod tests;
