//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-scope-runtime

use crate::server::AppState;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::PeerId;
use deve_core::protocol::{RepoListEntry, ServerMessage};

pub fn repo_list_message(
    state: &AppState,
    request_id: Option<String>,
    branch: Option<&PeerId>,
    scope_nonce: Option<u64>,
) -> anyhow::Result<ServerMessage> {
    let repos = state.repo.list_repos(branch)?;
    let repo_entries = if branch.is_none() {
        state
            .repo
            .list_local_repo_summaries()?
            .into_iter()
            .map(|summary| RepoListEntry {
                repo_id: summary.repo_id,
                name: summary.name,
                execution_name: summary.execution_name,
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok(ServerMessage::RepoList {
        request_id,
        branch: branch.map(ToString::to_string),
        scope_nonce,
        repos,
        repo_entries,
    })
}
