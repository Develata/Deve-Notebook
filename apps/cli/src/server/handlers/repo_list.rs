//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-scope-runtime

use crate::server::AppState;
use deve_core::models::PeerId;
use deve_core::protocol::{RepoListEntry, RepoReadiness, ServerMessage};

pub(crate) fn local_repo_list_entries(state: &AppState) -> anyhow::Result<Vec<RepoListEntry>> {
    state
        .repo
        .list_cataloged_local_repo_summaries()?
        .into_iter()
        .map(|summary| {
            let alias = state
                .repo
                .host_repo_alias_runtime()
                .binding(summary.repo_id)?;
            Ok(RepoListEntry {
                repo_id: summary.repo_id,
                display_alias: alias.alias,
                alias_revision: alias.alias_revision,
                readiness: state.watcher_runtime_view().repo_readiness(summary.repo_id),
            })
        })
        .collect()
}

pub(crate) fn remote_repo_list_entries(
    state: &AppState,
    branch: &PeerId,
) -> anyhow::Result<Vec<RepoListEntry>> {
    let entries = state
        .repo
        .list_remote_repo_ids(branch)?
        .into_iter()
        .map(|repo_id| RepoListEntry {
            repo_id,
            display_alias: repo_id.to_string(),
            alias_revision: 0,
            readiness: RepoReadiness::Readonly,
        })
        .collect::<Vec<_>>();
    Ok(entries)
}

pub fn repo_list_message(
    state: &AppState,
    request_id: Option<String>,
    branch: Option<&PeerId>,
    scope_nonce: Option<u64>,
) -> anyhow::Result<ServerMessage> {
    let repo_entries = match branch {
        None => local_repo_list_entries(state)?,
        Some(branch) => remote_repo_list_entries(state, branch)?,
    };
    Ok(ServerMessage::RepoList {
        request_id,
        branch: branch.map(ToString::to_string),
        scope_nonce,
        repo_entries,
    })
}
