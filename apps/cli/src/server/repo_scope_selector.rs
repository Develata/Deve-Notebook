use super::repo_scope_remote::recover_remote_repo_name_from_selector;
use crate::server::AppState;
use crate::server::session::WsSession;
use anyhow::{Result, anyhow};
use std::sync::Arc;

pub(super) fn resolve_repo_name_from_session(
    state: &Arc<AppState>,
    session: &WsSession,
) -> Result<Option<String>> {
    if session.active_branch.is_none() {
        return resolve_local_repo_name_from_session(state, session);
    }
    resolve_remote_repo_name_from_session(state, session)
}

fn resolve_local_repo_name_from_session(
    state: &Arc<AppState>,
    session: &WsSession,
) -> Result<Option<String>> {
    if let Some(repo_name) = session.active_repo.clone() {
        if uuid::Uuid::parse_str(&repo_name).is_ok() {
            return Err(anyhow!(
                "Local repository selector not resolved for {}",
                repo_name
            ));
        }
        let resolved = state
            .repo
            .resolve_local_repo_name_for_execution(None, Some(&repo_name))
            .map_err(|_| anyhow!("Local repository selector not resolved for {}", repo_name))?;
        if let Some(repo_id) = session.active_repo_id
            && state.repo.find_local_repo_name_by_id(repo_id)?.as_deref() != Some(resolved.as_str())
        {
            return Err(anyhow!(
                "Local repository selector not resolved for {}",
                repo_name
            ));
        }
        return Ok(Some(resolved));
    }
    if let Some(repo_id) = session.active_repo_id {
        return Err(anyhow!(
            "Local repository selector not resolved for {}",
            repo_id
        ));
    }
    Ok(None)
}

fn resolve_remote_repo_name_from_session(
    state: &Arc<AppState>,
    session: &WsSession,
) -> Result<Option<String>> {
    if let Some(repo_name) = session.active_repo.clone() {
        let Some(branch) = session.active_branch.as_ref() else {
            return Ok(Some(repo_name));
        };
        if let Some(selector) = recover_remote_repo_name_from_selector(
            state,
            branch,
            &repo_name,
            session.active_repo_id,
        )? {
            if selector != repo_name {
                tracing::warn!(
                    "Recovering remote repo selector from stale name: branch={}, stale_name={}, resolved_selector={}",
                    branch,
                    repo_name,
                    selector
                );
            }
            return Ok(Some(selector));
        }
        if let Some(repo_id) = session.active_repo_id
            && let Some(selector) = state.repo.find_remote_repo_selector_by_id(branch, repo_id)?
        {
            tracing::warn!(
                "Recovering remote repo selector from UUID after stale name miss: branch={}, repo_id={}, stale_name={}, resolved_selector={}",
                branch,
                repo_id,
                repo_name,
                selector
            );
            return Ok(Some(selector));
        }
        return Err(anyhow!(
            "Remote repository selector not resolved for {}",
            repo_name
        ));
    }
    let Some(repo_id) = session.active_repo_id else {
        return Ok(None);
    };
    let Some(branch) = session.active_branch.as_ref() else {
        return Err(anyhow!(
            "Local repository selector not resolved for {}",
            repo_id
        ));
    };
    if let Some(selector) = state.repo.find_remote_repo_selector_by_id(branch, repo_id)? {
        return Ok(Some(selector));
    }
    Err(anyhow!(
        "Remote session lost repo name for bound repo {}",
        repo_id
    ))
}
