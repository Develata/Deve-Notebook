//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 04_storage#facts-partition
//!   - 12_commands#cli-commands

use crate::admin_api::DumpResponse;
use deve_core::ledger::RepoManager;

pub fn build_dump(
    repo: &RepoManager,
    repo_name: &str,
    path: &str,
) -> anyhow::Result<Option<DumpResponse>> {
    let path = deve_core::utils::path::to_forward_slash(path);
    let (node_id, node_meta, doc_id) = repo.run_on_local_repo(repo_name, |db| {
        let node_id = deve_core::ledger::node_meta::get_node_id(db, &path)?;
        let node_meta = match node_id {
            Some(id) => deve_core::ledger::node_meta::get_node_meta(db, id)?,
            None => None,
        };
        let doc_id = node_meta.as_ref().and_then(|meta| meta.doc_id);
        Ok((node_id, node_meta, doc_id))
    })?;
    if node_id.is_none() && doc_id.is_none() {
        return Ok(None);
    }

    let structure_ops = match node_id {
        Some(node_id) => repo.get_local_structure_ops_in_local_repo(repo_name, node_id)?,
        None => Vec::new(),
    };
    let ops = match doc_id {
        Some(doc_id) => repo.get_local_ops_in_local_repo(repo_name, doc_id)?,
        None => Vec::new(),
    };
    let content = deve_core::state::reconstruct_content(
        &ops.iter()
            .map(|(_, entry)| entry.clone())
            .collect::<Vec<_>>(),
    );

    Ok(Some(DumpResponse {
        doc_id,
        node_id,
        node_meta,
        ops,
        structure_ops,
        content,
    }))
}
