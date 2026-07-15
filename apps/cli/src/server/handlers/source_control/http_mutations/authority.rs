//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! Source-control HTTP write authority checks.

use std::sync::Arc;

use crate::server::AppState;
use crate::server::auth::delegated_source_control::DELEGATED_SC_SCOPE_NONCE;
use crate::server::source_control_grants::AuthSessionId;
use deve_core::ledger::traits::RepoSelector;
use deve_core::models::RepoId;
use deve_core::protocol::{ServerError, ServerErrorCode};

pub(super) enum SourceControlWriteAuthority<'a> {
    BrowserSessionGrant(&'a AuthSessionId),
    DelegatedRemoteProxy,
}

#[derive(Clone, Debug)]
pub(super) struct AuthorizedRepoBinding {
    pub(super) repo_id: RepoId,
    repo_name: String,
}

impl AuthorizedRepoBinding {
    pub(super) fn pinned_selector(&self) -> RepoSelector {
        RepoSelector {
            repo_id: Some(self.repo_id),
            repo_name: Some(self.repo_name.clone()),
        }
    }
}

pub(super) fn authorize_http_write(
    state: &Arc<AppState>,
    selector: &RepoSelector,
    scope_nonce: u64,
    authority: SourceControlWriteAuthority<'_>,
) -> Result<AuthorizedRepoBinding, ServerError> {
    let writable_repo = resolve_http_writable_repo(state, selector)?;
    match authority {
        SourceControlWriteAuthority::BrowserSessionGrant(auth_session_id) => {
            state
                .source_control_write_grants()
                .authorize_browser_local(auth_session_id, writable_repo.repo_id, scope_nonce)?;
        }
        SourceControlWriteAuthority::DelegatedRemoteProxy => {
            if scope_nonce != DELEGATED_SC_SCOPE_NONCE {
                return Err(ServerError::with_detail(
                    ServerErrorCode::ScStaleScope,
                    "delegated source control scope nonce mismatch",
                ));
            }
        }
    }
    Ok(AuthorizedRepoBinding {
        repo_id: writable_repo.repo_id,
        repo_name: writable_repo.repo_name,
    })
}

struct HttpWritableRepo {
    repo_id: RepoId,
    repo_name: String,
}

fn resolve_http_writable_repo(
    state: &Arc<AppState>,
    selector: &RepoSelector,
) -> Result<HttpWritableRepo, ServerError> {
    let repo_name = state
        .repo
        .resolve_local_repo_name_for_execution(selector.repo_id, selector.repo_name.as_deref())
        .map_err(super::super::errors::map_repo_scope_error)?;
    let repo_id = state
        .repo
        .get_repo_info_for(None, Some(&repo_name))
        .map_err(super::super::errors::map_repo_scope_error)?
        .map(|info| info.uuid)
        .ok_or_else(|| {
            ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                "source control repo metadata missing",
            )
        })?;
    crate::server::repo_scope::ensure_local_repo_projection_writable(state, &repo_name)?;
    Ok(HttpWritableRepo { repo_id, repo_name })
}
