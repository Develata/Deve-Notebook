use crate::ledger::RepoManager;
use crate::models::{DocId, NodeKind};
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(super) struct ProjectionPlan {
    pub dirs: HashSet<String>,
    pub docs: HashMap<String, DocId>,
}

pub(super) fn build(repo: &RepoManager, repo_name: &str) -> Result<ProjectionPlan> {
    let mut dirs = HashSet::from([String::new()]);
    let mut docs = HashMap::new();

    for (_node_id, meta) in repo.list_local_nodes(Some(repo_name))? {
        let path = meta.path.trim_matches('/').to_string();
        if crate::utils::notegit::is_internal_repo_path(&path) {
            continue;
        }
        if meta.kind == NodeKind::Dir {
            if !path.is_empty() {
                dirs.insert(path.clone());
            }
            insert_parents(&mut dirs, &path);
            continue;
        }
        let Some(doc_id) = meta.doc_id else {
            continue;
        };
        insert_parents(&mut dirs, &path);
        docs.insert(path, doc_id);
    }

    Ok(ProjectionPlan { dirs, docs })
}

fn insert_parents(dirs: &mut HashSet<String>, path: &str) {
    let mut cursor = Path::new(path).parent();
    while let Some(parent) = cursor {
        let value = crate::utils::path::to_forward_slash(&parent.to_string_lossy());
        if value.is_empty() || !dirs.insert(value.clone()) {
            break;
        }
        cursor = parent.parent();
    }
}
