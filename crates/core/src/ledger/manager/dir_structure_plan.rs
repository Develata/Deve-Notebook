use super::dir_structure_support::{load_meta, plan_parent_chain};
use crate::ledger::RepoManager;
use crate::ledger::node_meta;
use crate::models::{NodeId, NodeKind, StructureOp};
use crate::utils::path::to_forward_slash;
use anyhow::{Result, anyhow};
pub(super) struct StructuredDirTarget {
    pub node_id: NodeId,
    pub ops: Vec<StructureOp>,
}

pub(super) fn plan_create(
    repo: &RepoManager,
    repo_name: &str,
    path: &str,
) -> Result<StructuredDirTarget> {
    let path = to_forward_slash(path);
    if let Some(node_id) =
        repo.run_on_local_repo(repo_name, |db| node_meta::get_node_id(db, &path))?
    {
        let meta = load_meta(repo, repo_name, node_id)?;
        if meta.kind != NodeKind::Dir {
            return Err(anyhow!("target path is not a directory: {}", path));
        }
        return Ok(StructuredDirTarget {
            node_id,
            ops: Vec::new(),
        });
    }
    let (mut ops, parent_id, name) = plan_parent_chain(repo, repo_name, &path)?;
    let node_id = NodeId::new();
    ops.push(StructureOp::CreateDir {
        node_id,
        parent_id,
        name,
    });
    Ok(StructuredDirTarget { node_id, ops })
}

pub(super) fn plan_rename(
    repo: &RepoManager,
    repo_name: &str,
    old_path: &str,
    new_path: &str,
) -> Result<Option<StructuredDirTarget>> {
    let old_path = to_forward_slash(old_path);
    let Some(node_id) =
        repo.run_on_local_repo(repo_name, |db| node_meta::get_node_id(db, &old_path))?
    else {
        return Ok(None);
    };
    let meta = load_meta(repo, repo_name, node_id)?;
    let (mut ops, parent_id, name) = plan_parent_chain(repo, repo_name, new_path)?;
    if meta.parent_id != parent_id {
        ops.push(StructureOp::MoveNode {
            node_id,
            doc_id: None,
            new_parent_id: parent_id,
        });
    }
    if meta.name != name {
        ops.push(StructureOp::RenameNode {
            node_id,
            doc_id: None,
            new_name: name,
        });
    }
    Ok(Some(StructuredDirTarget { node_id, ops }))
}

pub(super) fn plan_delete(
    repo: &RepoManager,
    repo_name: &str,
    path: &str,
) -> Result<Option<StructuredDirTarget>> {
    let path = to_forward_slash(path);
    let Some(node_id) =
        repo.run_on_local_repo(repo_name, |db| node_meta::get_node_id(db, &path))?
    else {
        return Ok(None);
    };
    Ok(Some(StructuredDirTarget {
        node_id,
        ops: vec![StructureOp::DeleteNode {
            node_id,
            doc_id: None,
        }],
    }))
}
