//! plan_ref:
//!   - 04_storage.md §Non-Negotiable Invariants

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
        let rebuilt = rebuild::rebuild_local_doc_in_repo(repo, repo_name, doc_id)?;
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
