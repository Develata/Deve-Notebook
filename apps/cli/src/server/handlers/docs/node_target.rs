use crate::server::AppState;
use crate::server::repo_scope::{ResolvedRepo, local_repo_path, run_on_resolved_local_repo};
use anyhow::{Result, anyhow};
use deve_core::ledger::node_meta;
use deve_core::models::{DocId, NodeId, NodeKind};
use std::path::PathBuf;
use std::sync::Arc;

pub(super) struct NodeTarget {
    pub node_id: NodeId,
    pub kind: NodeKind,
    pub doc_id: Option<DocId>,
    pub repo_path: String,
    pub abs_path: PathBuf,
}

pub(super) fn resolve_node_target(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    path: &str,
) -> Result<Option<NodeTarget>> {
    let meta = run_on_resolved_local_repo(state, scope, |db| {
        let Some(node_id) = node_meta::get_node_id(db, path)? else {
            return Ok(None);
        };
        let meta =
            node_meta::get_node_meta(db, node_id)?.ok_or_else(|| anyhow!("Node meta missing"))?;
        Ok(Some((node_id, meta.kind, meta.doc_id, meta.path)))
    })?;
    meta.map(|(node_id, kind, doc_id, repo_path)| {
        let abs_path = local_repo_path(state, scope, &repo_path)
            .map_err(|_| anyhow!("Canonical node path escaped local repo: {}", repo_path))?;
        Ok(NodeTarget {
            node_id,
            kind,
            doc_id,
            repo_path,
            abs_path,
        })
    })
    .transpose()
}
