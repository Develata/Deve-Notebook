//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 09_web_thin_client_ledger#document-create-intent
//!
//! Exact proposed-identity admission for idempotent document structure Create.

use crate::ledger::{RepoManager, node_meta};
use crate::models::{DocId, NodeId};
use crate::utils::path::to_forward_slash;
use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureCreateIdentityState {
    Vacant,
    Exact { doc_id: Option<DocId> },
    Conflict,
}

impl RepoManager {
    pub fn inspect_structure_create_identity_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
        proposed_node_id: NodeId,
        expected_doc_id: Option<DocId>,
    ) -> Result<StructureCreateIdentityState> {
        let normalized = to_forward_slash(path);
        self.run_on_local_repo(repo_name, |db| {
            let path_node_id = node_meta::get_node_id(db, &normalized)?;
            let proposed_meta = node_meta::get_node_meta(db, proposed_node_id)?;
            match (path_node_id, proposed_meta) {
                (None, None) => Ok(StructureCreateIdentityState::Vacant),
                (Some(path_node_id), Some(meta))
                    if path_node_id == proposed_node_id
                        && meta.path == normalized
                        && meta.doc_id == expected_doc_id =>
                {
                    Ok(StructureCreateIdentityState::Exact {
                        doc_id: meta.doc_id,
                    })
                }
                _ => Ok(StructureCreateIdentityState::Conflict),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DocId;

    #[test]
    fn document_create_identity_is_exact_only_for_same_uuid_path_and_kind() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let (repo, _) = crate::test_support::init_cataloged_repo_with_depth(
            &dir.path().join("ledger"),
            &dir.path().join("notes"),
            2,
        )?;
        let repo_name = repo.local_repo_name().to_string();
        let doc_id = DocId::new();
        let node_id = NodeId::from_doc_id(doc_id);

        assert_eq!(
            repo.inspect_structure_create_identity_in_local_repo(
                &repo_name,
                "notes/a.md",
                node_id,
                Some(doc_id),
            )?,
            StructureCreateIdentityState::Vacant
        );
        repo.apply_file_structure_in_local_repo(&repo_name, "notes/a.md", Some(doc_id), "test")?;
        assert_eq!(
            repo.inspect_structure_create_identity_in_local_repo(
                &repo_name,
                "notes/a.md",
                node_id,
                Some(doc_id),
            )?,
            StructureCreateIdentityState::Exact {
                doc_id: Some(doc_id)
            }
        );
        assert_eq!(
            repo.inspect_structure_create_identity_in_local_repo(
                &repo_name,
                "notes/b.md",
                node_id,
                Some(doc_id),
            )?,
            StructureCreateIdentityState::Conflict
        );
        assert_eq!(
            repo.inspect_structure_create_identity_in_local_repo(
                &repo_name,
                "notes/a.md",
                NodeId::new(),
                None,
            )?,
            StructureCreateIdentityState::Conflict
        );
        Ok(())
    }
}
