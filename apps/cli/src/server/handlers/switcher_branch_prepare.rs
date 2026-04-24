//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!
//! Branch switch target repo preparation.

use super::super::switcher_error::prepare_switch_error;
use super::super::switcher_prepare::{PreparedRepoSwitch, prepare_repo_switch};
use crate::server::AppState;
use deve_core::models::PeerId;
use deve_core::protocol::ServerError;
use std::sync::Arc;

pub(super) fn prepare_target_repo_switch(
    state: &Arc<AppState>,
    target_branch: Option<&PeerId>,
    target_repo: Option<String>,
) -> Result<Option<PreparedRepoSwitch>, ServerError> {
    let Some(repo_name) = target_repo else {
        return Ok(None);
    };
    tracing::info!("Auto-switching to repo: {}", repo_name);
    prepare_repo_switch(state, target_branch, repo_name)
        .map(Some)
        .map_err(|err| prepare_switch_error(target_branch, err))
}
