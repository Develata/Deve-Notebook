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
    pub abs_path: PathBuf,
}

pub(super) fn resolve_node_target(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    path: &str,
) -> Result<Option<NodeTarget>> {
    let abs_path = local_repo_path(state, scope, path)?;
    let meta = run_on_resolved_local_repo(state, scope, |db| {
        let Some(node_id) = node_meta::get_node_id(db, path)? else {
            return Ok(None);
        };
        let meta =
            node_meta::get_node_meta(db, node_id)?.ok_or_else(|| anyhow!("Node meta missing"))?;
        Ok(Some((node_id, meta.kind, meta.doc_id)))
    })?;
    Ok(meta.map(|(node_id, kind, doc_id)| NodeTarget {
        node_id,
        kind,
        doc_id,
        abs_path,
    }))
}
