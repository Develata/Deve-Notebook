use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
#[path = "switcher_branch.rs"]
mod switcher_branch;
#[path = "switcher_error.rs"]
mod switcher_error;
#[path = "switcher_guard.rs"]
mod switcher_guard;
#[path = "switcher_payload.rs"]
mod switcher_payload;
#[path = "switcher_prepare.rs"]
mod switcher_prepare;
#[cfg(test)]
#[path = "switcher_prepare_test/mod.rs"]
mod switcher_prepare_test;
#[path = "switcher_repo.rs"]
mod switcher_repo;
#[path = "switcher_scope.rs"]
mod switcher_scope;
#[path = "switcher_selector.rs"]
mod switcher_selector;

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
#[path = "../switcher_switch_nonce_test.rs"]
mod switcher_switch_nonce_test;
