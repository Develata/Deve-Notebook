//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Branch switch preparation.

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::map_repo_scope_error;
use anyhow::anyhow;
use deve_core::ledger::listing::RepoListing;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

pub(crate) fn validate_branch_target(
    state: &Arc<AppState>,
    ch: &DualChannel,
    peer_id: &Option<String>,
    switch_nonce: Option<u64>,
) -> Option<Option<String>> {
    let Some(pid_str) = peer_id else {
        return Some(None);
    };
    let shadows = match state.repo.list_switchable_shadows_on_disk() {
        Ok(shadows) => shadows,
        Err(err) => {
            ch.send_protocol_error_with_switch_nonce(
                map_repo_scope_error(anyhow!("Failed to list shadows: {}", err)),
                switch_nonce,
            );
            return None;
        }
    };
    let local_repos = match state.repo.list_local_repo_names_for_execution() {
        Ok(repos) => repos,
        Err(err) => {
            ch.send_protocol_error_with_switch_nonce(
                map_repo_scope_error(anyhow!("Failed to list local repos: {}", err)),
                switch_nonce,
            );
            return None;
        }
    };
    let is_valid_shadow = shadows.iter().any(|p| p.as_str() == pid_str);
    let is_local_repo = local_repos.contains(pid_str);
    if !is_valid_shadow && is_local_repo {
        ch.send_protocol_error_with_switch_nonce(
            ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                format!(
                    "SwitchBranch expects a shadow peer, got local repo selector: {}",
                    pid_str
                ),
            ),
            switch_nonce,
        );
        return None;
    }
    if !is_valid_shadow {
        ch.send_protocol_error_with_switch_nonce(
            ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                format!("Shadow branch not found: {}", pid_str),
            ),
            switch_nonce,
        );
        return None;
    }
    Some(peer_id.clone())
}
