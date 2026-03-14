use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::state::reconstruct_content;
use anyhow::Result;

pub(super) fn rebuild_doc_projection(
    repo: &RepoManager,
    repo_name: &str,
    doc_id: DocId,
) -> Result<String> {
    let ops = repo.get_local_ops_in_local_repo(repo_name, doc_id)?;
    let entries: Vec<_> = ops.into_iter().map(|(_, entry)| entry).collect();
    Ok(reconstruct_content(&entries))
}
