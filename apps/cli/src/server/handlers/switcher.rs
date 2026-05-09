//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Repository switcher WebSocket handler facade.

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
mod switcher_branch;
mod switcher_branch_hint;
mod switcher_error;
mod switcher_guard;
#[cfg(test)]
mod switcher_last_local_repo_test;
mod switcher_payload;
mod switcher_prepare;
#[cfg(test)]
mod switcher_prepare_test;
mod switcher_repo;
mod switcher_scope;
mod switcher_selector;
#[cfg(test)]
mod switcher_selector_single_remote_test;

use deve_core::models::RepoId;
use std::sync::Arc;

pub(crate) use self::switcher_payload::{
    RepoViewPayload, emit_repo_view, prepare_repo_view_messages, switch_scope_nonce,
};

pub async fn handle_switch_branch(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    peer_id: Option<String>,
    switch_nonce: Option<u64>,
) {
    switcher_branch::handle_switch_branch(state, ch, session, peer_id, switch_nonce).await;
}

pub async fn handle_switch_repo(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    name: String,
    repo_id: Option<RepoId>,
    switch_nonce: Option<u64>,
) {
    switcher_repo::handle_switch_repo(state, ch, session, name, repo_id, switch_nonce).await;
}

#[cfg(test)]
mod switcher_switch_nonce_test;
