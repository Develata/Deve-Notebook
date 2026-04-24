//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Repo switch payload preloading.

use super::switcher_prepare::PreparedRepoSwitch;
use crate::server::AppState;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::node_meta;
use deve_core::models::{DocId, NodeId, NodeMeta, PeerId, RepoId, RepoType};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

#[path = "switcher_payload_emit.rs"]
mod emit;
#[path = "switcher_payload_local.rs"]
mod local;

pub(crate) struct RepoViewPayload {
    pub repo_name: String,
    pub repo_id: RepoId,
    pub docs: Vec<(DocId, String)>,
    pub nodes: Vec<(NodeId, NodeMeta)>,
}

pub(crate) struct RepoViewMessages {
    pub repo_switched: ServerMessage,
    pub doc_list: ServerMessage,
    pub tree_update: ServerMessage,
}

pub(super) struct SwitchPayload {
    pub repo_list: Vec<String>,
    pub repo_view: Option<RepoViewPayload>,
}

pub(super) fn preload_branch_switch(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    prepared: Option<&PreparedRepoSwitch>,
) -> anyhow::Result<SwitchPayload> {
    let repo_list = state.repo.list_repos(branch)?;
    let repo_view = prepared
        .map(|prepared| load_repo_view(state, branch, prepared))
        .transpose()?;
    Ok(SwitchPayload {
        repo_list,
        repo_view,
    })
}

pub(super) fn preload_repo_view(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    prepared: &PreparedRepoSwitch,
) -> anyhow::Result<RepoViewPayload> {
    load_repo_view(state, branch, prepared)
}
pub(crate) use emit::{emit_repo_view, prepare_repo_view_messages, switch_scope_nonce};

fn load_repo_view(
    state: &Arc<AppState>,
    branch: Option<&PeerId>,
    prepared: &PreparedRepoSwitch,
) -> anyhow::Result<RepoViewPayload> {
    let repo_id = prepared.repo_id.ok_or_else(|| {
        anyhow::anyhow!("Repository UUID not resolved for {}", prepared.repo_name)
    })?;
    let (docs, nodes) = if let Some(handle) = prepared.db.as_ref() {
        (
            node_meta::list_file_docs(&handle.db)?,
            node_meta::list_nodes(&handle.db)?,
        )
    } else if let Some(peer_id) = branch.cloned() {
        (
            state
                .repo
                .list_docs(&RepoType::Remote(peer_id.clone(), repo_id))?,
            state.repo.list_nodes(&RepoType::Remote(peer_id, repo_id))?,
        )
    } else {
        local::load_local_repo_view(state, prepared)?
    };
    Ok(RepoViewPayload {
        repo_name: prepared.repo_name.clone(),
        repo_id,
        docs,
        nodes,
    })
}
