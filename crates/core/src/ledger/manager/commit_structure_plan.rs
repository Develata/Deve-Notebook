//! plan_ref:
//!   - 04_repository#tree-projection-contract

use crate::ledger::RepoManager;
use crate::ledger::node_meta;
use crate::models::{DocId, NodeId, NodeKind, StructureOp};
use crate::utils::path::to_forward_slash;
use anyhow::{Result, anyhow};

pub(super) struct StructuredCommitTarget {
    pub doc_id: DocId,
    pub ops: Vec<StructureOp>,
}

pub(super) fn plan_file_upsert(
    repo: &RepoManager,
    repo_name: &str,
    path: &str,
    doc_id_hint: Option<DocId>,
) -> Result<StructuredCommitTarget> {
    let path = to_forward_slash(path);
    let doc_id = resolve_doc_id(repo, repo_name, &path, doc_id_hint)?;
    let (mut ops, parent_id, name) = plan_parent_chain(repo, repo_name, &path)?;
    let Some(meta) = current_meta(repo, repo_name, doc_id)? else {
        ops.push(StructureOp::CreateFile {
            node_id: NodeId::from_doc_id(doc_id),
            doc_id,
            parent_id,
            name,
        });
        return Ok(StructuredCommitTarget { doc_id, ops });
    };
    if meta.parent_id != parent_id {
        ops.push(StructureOp::MoveNode {
            node_id: NodeId::from_doc_id(doc_id),
            doc_id: Some(doc_id),
            new_parent_id: parent_id,
        });
    }
    if meta.name != name {
        ops.push(StructureOp::RenameNode {
            node_id: NodeId::from_doc_id(doc_id),
            doc_id: Some(doc_id),
            new_name: name,
        });
    }
    Ok(StructuredCommitTarget { doc_id, ops })
}

pub(super) fn plan_delete(
    repo: &RepoManager,
    repo_name: &str,
    path: &str,
    doc_id_hint: Option<DocId>,
) -> Result<Option<StructuredCommitTarget>> {
    let path = to_forward_slash(path);
    let doc_id = match doc_id_hint {
        Some(doc_id) => {
            let meta = repo
                .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)?
                .ok_or_else(|| anyhow!("source control delete target doc not found: {}", doc_id))?;
            let current_path = to_forward_slash(&meta.path);
            if current_path != path {
                return Err(anyhow!(
                    "source control delete target path mismatch: doc {} is at {}, staged path {}",
                    doc_id,
                    current_path,
                    path
                ));
            }
            Some(doc_id)
        }
        None => repo.get_tracked_docid_in_local_repo(repo_name, &path)?,
    };
    Ok(doc_id.map(|doc_id| StructuredCommitTarget {
        doc_id,
        ops: vec![StructureOp::DeleteNode {
            node_id: NodeId::from_doc_id(doc_id),
            doc_id: Some(doc_id),
        }],
    }))
}

fn resolve_doc_id(
    repo: &RepoManager,
    repo_name: &str,
    path: &str,
    doc_id_hint: Option<DocId>,
) -> Result<DocId> {
    if let Some(doc_id) = doc_id_hint {
        return Ok(doc_id);
    }
    if let Some(doc_id) = repo.get_tracked_docid_in_local_repo(repo_name, path)? {
        return Ok(doc_id);
    }
    Ok(DocId::new())
}

fn current_meta(
    repo: &RepoManager,
    repo_name: &str,
    doc_id: DocId,
) -> Result<Option<crate::models::NodeMeta>> {
    repo.get_file_meta_for_doc_in_local_repo(repo_name, doc_id)
}

fn plan_parent_chain(
    repo: &RepoManager,
    repo_name: &str,
    path: &str,
) -> Result<(Vec<StructureOp>, Option<NodeId>, String)> {
    let path = to_forward_slash(path);
    let (parent_path, name) = path
        .rfind('/')
        .map_or(("", path.as_str()), |idx| (&path[..idx], &path[idx + 1..]));
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
            let meta = repo.run_on_local_repo(repo_name, |db| {
                node_meta::get_node_meta(db, node_id)?
                    .ok_or_else(|| anyhow!("node meta missing for {}", current))
            })?;
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
