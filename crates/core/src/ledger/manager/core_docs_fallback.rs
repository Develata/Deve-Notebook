//! plan_ref:
//!   - 04_repository#tree-projection-contract

use anyhow::Result;
use sha2::{Digest, Sha256};

use crate::ledger::manager::types::RepoManager;
use crate::ledger::metadata;
use crate::models::{DocId, NodeId, NodeKind, NodeMeta};
use crate::utils::path::to_forward_slash;

impl RepoManager {
    pub fn list_local_docs_from_metadata_projection(
        &self,
        repo_name: Option<&str>,
    ) -> Result<Vec<(DocId, String)>> {
        let name = repo_name.unwrap_or(&self.local_repo_name);
        self.run_on_local_repo(name, metadata::list_docs)
    }

    pub fn list_local_nodes_from_metadata_projection(
        &self,
        repo_name: Option<&str>,
    ) -> Result<Vec<(NodeId, NodeMeta)>> {
        let docs = self.list_local_docs_from_metadata_projection(repo_name)?;
        Ok(build_nodes_from_docs(docs))
    }
}

fn build_nodes_from_docs(docs: Vec<(DocId, String)>) -> Vec<(NodeId, NodeMeta)> {
    let mut nodes = Vec::new();
    let mut dir_paths = std::collections::BTreeSet::<String>::new();

    for (_, raw_path) in &docs {
        let path = to_forward_slash(raw_path);
        let mut cursor = std::path::Path::new(&path).parent();
        while let Some(parent) = cursor {
            let normalized = to_forward_slash(&parent.to_string_lossy());
            if normalized.is_empty() {
                break;
            }
            dir_paths.insert(normalized.clone());
            cursor = parent.parent();
        }
    }

    for dir_path in dir_paths {
        let parent_id = parent_node_id(&dir_path);
        nodes.push((
            dir_node_id(&dir_path),
            NodeMeta {
                kind: NodeKind::Dir,
                name: path_name(&dir_path),
                parent_id,
                path: dir_path,
                doc_id: None,
            },
        ));
    }

    for (doc_id, raw_path) in docs {
        let path = to_forward_slash(&raw_path);
        nodes.push((
            NodeId::from_doc_id(doc_id),
            NodeMeta {
                kind: NodeKind::File,
                name: path_name(&path),
                parent_id: parent_node_id(&path),
                path,
                doc_id: Some(doc_id),
            },
        ));
    }

    nodes
}

fn parent_node_id(path: &str) -> Option<NodeId> {
    let parent = std::path::Path::new(path).parent()?;
    let normalized = to_forward_slash(&parent.to_string_lossy());
    (!normalized.is_empty()).then(|| dir_node_id(&normalized))
}

fn dir_node_id(path: &str) -> NodeId {
    let digest = Sha256::digest(path.as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    NodeId(uuid::Uuid::from_bytes(bytes))
}

fn path_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::build_nodes_from_docs;
    use crate::models::{DocId, NodeKind};

    #[test]
    fn rebuilds_directory_nodes_from_doc_paths() {
        let docs = vec![
            (DocId::new(), "notes/a.md".to_string()),
            (DocId::new(), "notes/nested/b.md".to_string()),
        ];
        let nodes = build_nodes_from_docs(docs);
        let paths = nodes
            .iter()
            .map(|(_, meta)| (meta.path.clone(), meta.kind))
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(paths.get("notes"), Some(&NodeKind::Dir));
        assert_eq!(paths.get("notes/nested"), Some(&NodeKind::Dir));
        assert_eq!(paths.get("notes/a.md"), Some(&NodeKind::File));
        assert_eq!(paths.get("notes/nested/b.md"), Some(&NodeKind::File));
    }
}
