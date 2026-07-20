//! plan_ref:
//!   - 03_storage/projection#projection-contract

use super::super::{projection_plan, rebuild};
use super::EntryKind;
use crate::ledger::RepoManager;
use crate::source_control::pending_fs;
use anyhow::Result;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct ProjectedEntry {
    pub kind: EntryKind,
    pub content_hash: Option<String>,
}

pub fn enumerate_projection(
    repo: &RepoManager,
    repo_name: &str,
) -> Result<BTreeMap<String, ProjectedEntry>> {
    let plan = projection_plan::build(repo, repo_name)?;
    enumerate_projection_from_plan(plan, |doc_id| {
        rebuild::rebuild_local_doc_in_repo(repo, repo_name, doc_id)
    })
}

fn enumerate_projection_from_plan(
    plan: projection_plan::ProjectionPlan,
    mut rebuild_doc: impl FnMut(crate::models::DocId) -> Result<rebuild::RebuildResult>,
) -> Result<BTreeMap<String, ProjectedEntry>> {
    let mut entries = BTreeMap::new();

    for path in plan.dirs {
        if path.is_empty() {
            continue;
        }
        entries.insert(
            path,
            ProjectedEntry {
                kind: EntryKind::Dir,
                content_hash: None,
            },
        );
    }

    for (path, doc_id) in plan.docs {
        let rebuilt = rebuild_doc(doc_id)?;
        entries.insert(
            path,
            ProjectedEntry {
                kind: EntryKind::File,
                content_hash: Some(pending_fs::content_hash(&rebuilt.content)),
            },
        );
    }

    Ok(entries)
}
