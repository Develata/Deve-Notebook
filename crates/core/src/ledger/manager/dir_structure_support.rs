use crate::ledger::RepoManager;
use crate::ledger::node_meta;
use crate::models::{DocId, NodeId, NodeKind, StructureOp};
use anyhow::{Result, anyhow};

pub(super) fn dir_event_doc_id(node_id: NodeId) -> DocId {
    // Route 2 过渡期：目录结构事件仍需落在 `LedgerEntry.doc_id` 上，
    // 这里用目录的 `NodeId` 派生一个稳定的 synthetic doc id。
    DocId::from_u128(node_id.as_u128())
}

pub(super) fn load_meta(
    repo: &RepoManager,
    repo_name: &str,
    node_id: NodeId,
) -> Result<crate::models::NodeMeta> {
    repo.run_on_local_repo(repo_name, |db| {
        node_meta::get_node_meta(db, node_id)?
            .ok_or_else(|| anyhow!("node meta missing for {}", node_id))
    })
}

pub(super) fn plan_parent_chain(
    repo: &RepoManager,
    repo_name: &str,
    path: &str,
) -> Result<(Vec<StructureOp>, Option<NodeId>, String)> {
    let (parent_path, name) = path
        .rfind('/')
        .map_or(("", path), |idx| (&path[..idx], &path[idx + 1..]));
    let mut ops = Vec::new();
    let mut parent_id = None;
    let mut current = String::new();
    for segment in parent_path.split('/').filter(|part| !part.is_empty()) {
        if !current.is_empty() {
            current.push('/');
        }
        current.push_str(segment);
        if let Some(node_id) =
            repo.run_on_local_repo(repo_name, |db| node_meta::get_node_id(db, &current))?
        {
            let meta = load_meta(repo, repo_name, node_id)?;
            if meta.kind != NodeKind::Dir {
                return Err(anyhow!("target parent is not a directory: {}", current));
            }
            parent_id = Some(node_id);
            continue;
        }
        let node_id = NodeId::new();
        ops.push(StructureOp::CreateDir {
            node_id,
            parent_id,
            name: segment.to_string(),
        });
        parent_id = Some(node_id);
    }
    Ok((ops, parent_id, name.to_string()))
}
