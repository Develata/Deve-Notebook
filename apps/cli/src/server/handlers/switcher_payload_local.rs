use crate::server::AppState;
use deve_core::models::{NodeId, NodeMeta};
use std::sync::Arc;

use super::PreparedRepoSwitch;

pub(super) fn load_local_repo_view(
    state: &Arc<AppState>,
    prepared: &PreparedRepoSwitch,
) -> anyhow::Result<(
    Vec<(deve_core::models::DocId, String)>,
    Vec<(NodeId, NodeMeta)>,
)> {
    if prepared.degraded_docs_only {
        let docs = state
            .repo
            .list_local_docs_from_metadata_projection(Some(&prepared.repo_name))?;
        let nodes = state
            .repo
            .list_local_nodes_from_metadata_projection(Some(&prepared.repo_name))?;
        return Ok((docs, nodes));
    }
    Ok((
        state.repo.list_local_docs(Some(&prepared.repo_name))?,
        state.repo.list_local_nodes(Some(&prepared.repo_name))?,
    ))
}
