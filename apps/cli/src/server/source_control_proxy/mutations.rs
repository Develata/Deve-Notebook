//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime

use super::http::ProxyScOp;
use super::{RemoteSourceControlApi, http};
use anyhow::Result;
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::ScPathTarget;
use serde_json::json;

use super::REMOTE_PROXY_SCOPE_NONCE;

pub(super) fn stage_pending(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    target: &ScPathTarget,
) -> Result<()> {
    post_target(
        api,
        repo,
        "/api/sc/stage-pending",
        target,
        ProxyScOp::StagePending(target.path.clone()),
    )
}

pub(super) fn discard_pending(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    target: &ScPathTarget,
) -> Result<()> {
    post_target(
        api,
        repo,
        "/api/sc/discard-pending",
        target,
        ProxyScOp::DiscardPending(target.path.clone()),
    )
}

pub(super) fn unstage_file(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    target: &ScPathTarget,
) -> Result<()> {
    post_target(
        api,
        repo,
        "/api/sc/unstage",
        target,
        ProxyScOp::Unstage(target.path.clone()),
    )
}

fn post_target(
    api: &RemoteSourceControlApi,
    repo: &RepoSelector,
    route: &str,
    target: &ScPathTarget,
    op: ProxyScOp,
) -> Result<()> {
    let url = format!("{}{}", api.base_url, route);
    super::block_on_safe(async {
        http::send_empty_with_op(
            api.client.post(&url).json(&json!({
                "scope_nonce": REMOTE_PROXY_SCOPE_NONCE,
                "path": target.path,
                "doc_id": target.doc_id.map(|id| id.to_string()),
                "repo_id": repo.repo_id.map(|id| id.to_string()),
                "repo_name": repo.repo_name.clone(),
            })),
            op,
        )
        .await
    })?;
    Ok(())
}
