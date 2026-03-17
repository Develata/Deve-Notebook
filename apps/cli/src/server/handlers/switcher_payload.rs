use super::switcher_prepare::PreparedRepoSwitch;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::node_meta;
use deve_core::models::{DocId, NodeId, NodeMeta, PeerId, RepoId, RepoType};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

pub(super) struct RepoViewPayload {
    pub repo_name: String,
    pub repo_id: RepoId,
    pub docs: Vec<(DocId, String)>,
    pub nodes: Vec<(NodeId, NodeMeta)>,
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

pub(super) fn emit_repo_view(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    branch: Option<String>,
    switch_nonce: Option<u64>,
    repo_view: Option<RepoViewPayload>,
) {
    let Some(repo_view) = repo_view else {
        return;
    };
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let tree_branch = branch.clone().map(PeerId::new);
    ch.unicast(ServerMessage::RepoSwitched {
        branch: branch.clone(),
        name: repo_view.repo_name,
        uuid: repo_view.repo_id.to_string(),
        switch_nonce,
    });
    ch.unicast(ServerMessage::DocList {
        request_id: None,
        repo_id: Some(repo_view.repo_id),
        branch: tree_branch.clone(),
        scope_nonce,
        docs: repo_view.docs,
    });
    let delta = state.tree_manager.reset_from_nodes(
        repo_view.repo_id,
        tree_branch.as_ref(),
        repo_view.nodes,
    );
    ch.unicast(ServerMessage::TreeUpdate {
        request_id: None,
        repo_id: Some(repo_view.repo_id),
        branch: tree_branch,
        scope_nonce,
        delta,
    });
}

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
        (
            state.repo.list_local_docs(Some(&prepared.repo_name))?,
            state.repo.list_local_nodes(Some(&prepared.repo_name))?,
        )
    };
    Ok(RepoViewPayload {
        repo_name: prepared.repo_name.clone(),
        repo_id,
        docs,
        nodes,
    })
}
