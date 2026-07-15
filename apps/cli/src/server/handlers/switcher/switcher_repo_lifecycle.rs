//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-health-and-repair

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::repo_list::repo_list_message;
use crate::server::repo_mutation::{MutationExecution, MutationPublication};
use crate::server::session::WsSession;
use deve_core::models::RepoId;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

pub(super) async fn handle_rename_repo(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    repo_id: RepoId,
    raw_name: String,
    switch_nonce: Option<u64>,
) {
    if !super::switcher_guard::require_browser_switch_nonce(
        ch,
        session,
        switch_nonce,
        "repo rename",
    ) {
        return;
    }
    if session.active_branch.is_some() {
        ch.send_protocol_error_with_switch_nonce(
            invalid_repo_context("Cannot rename a local repository while viewing a remote branch"),
            switch_nonce,
        );
        return;
    }
    let name = raw_name.trim().to_string();
    if name.is_empty() {
        ch.send_protocol_error_with_switch_nonce(
            invalid_repo_context("Repository name must not be empty"),
            switch_nonce,
        );
        return;
    }

    let execution = state
        .repo_mutation_gate()
        .execute_catalog_repo(repo_id, &state.tx, || {
            match state.repo.rename_local_repo(repo_id, &name) {
                Ok(summary) => MutationExecution::committed(
                    summary,
                    MutationPublication::document_recovery(
                        repo_id,
                        deve_core::protocol::DocumentRecoveryScope::CurrentDocument,
                    ),
                ),
                Err(error) => MutationExecution::committed_partial(
                    error,
                    MutationPublication::document_recovery(
                        repo_id,
                        deve_core::protocol::DocumentRecoveryScope::CurrentDocument,
                    ),
                ),
            }
        })
        .await;
    let summary = match execution {
        Ok(MutationExecution::Committed { value, .. }) => value,
        Ok(MutationExecution::NotCommitted(err))
        | Ok(MutationExecution::ProjectionDegraded { error: err, .. })
        | Ok(MutationExecution::CommittedPartial { error: err, .. }) => {
            ch.send_protocol_error_with_switch_nonce(
                ServerError::with_detail(
                    ServerErrorCode::StoragePersistFailed,
                    format!("Failed to rename repository: {err}"),
                ),
                switch_nonce,
            );
            return;
        }
        Err(error) => {
            ch.send_protocol_error_with_switch_nonce(
                ServerError::with_detail(
                    ServerErrorCode::StoragePersistFailed,
                    format!("Failed to serialize repository rename: {error}"),
                ),
                switch_nonce,
            );
            return;
        }
    };

    emit_repo_list(state, ch, session, switch_nonce);
    if active_repo_id(state, session) == Some(repo_id) {
        super::switcher_repo::handle_switch_repo(
            state,
            ch,
            session,
            summary.name,
            Some(repo_id),
            switch_nonce,
        )
        .await;
    }
}

pub(super) async fn handle_remove_repo(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    repo_id: RepoId,
    switch_nonce: Option<u64>,
) {
    if !super::switcher_guard::require_browser_switch_nonce(
        ch,
        session,
        switch_nonce,
        "repo remove",
    ) {
        return;
    }
    if session.active_branch.is_some() {
        ch.send_protocol_error_with_switch_nonce(
            invalid_repo_context("Cannot remove a local repository while viewing a remote branch"),
            switch_nonce,
        );
        return;
    }

    let current_repo_id = active_repo_id(state, session);
    let fallback = if current_repo_id == Some(repo_id) {
        match state
            .repo
            .list_local_repo_summaries()
            .and_then(|summaries| {
                summaries
                    .into_iter()
                    .find(|summary| summary.repo_id != repo_id)
                    .ok_or_else(|| anyhow::anyhow!("Cannot remove the last local repository"))
            }) {
            Ok(summary) => Some(summary),
            Err(err) => {
                ch.send_protocol_error_with_switch_nonce(
                    invalid_repo_context(format!("Failed to select fallback repository: {err}")),
                    switch_nonce,
                );
                return;
            }
        }
    } else {
        None
    };

    let execution = state
        .repo_mutation_gate()
        .execute_catalog_repo(repo_id, &state.tx, || {
            match state.repo.remove_local_repo(repo_id) {
                Ok(summary) => MutationExecution::committed(
                    summary,
                    MutationPublication::document_recovery(
                        repo_id,
                        deve_core::protocol::DocumentRecoveryScope::CurrentDocument,
                    ),
                ),
                Err(error) => MutationExecution::committed_partial(
                    error,
                    MutationPublication::document_recovery(
                        repo_id,
                        deve_core::protocol::DocumentRecoveryScope::CurrentDocument,
                    ),
                ),
            }
        })
        .await;
    match execution {
        Ok(MutationExecution::Committed { .. }) => {}
        Ok(MutationExecution::NotCommitted(err))
        | Ok(MutationExecution::ProjectionDegraded { error: err, .. })
        | Ok(MutationExecution::CommittedPartial { error: err, .. }) => {
            ch.send_protocol_error_with_switch_nonce(
                ServerError::with_detail(
                    ServerErrorCode::StoragePersistFailed,
                    format!("Failed to remove repository: {err}"),
                ),
                switch_nonce,
            );
            return;
        }
        Err(error) => {
            ch.send_protocol_error_with_switch_nonce(
                ServerError::with_detail(
                    ServerErrorCode::StoragePersistFailed,
                    format!("Failed to serialize repository removal: {error}"),
                ),
                switch_nonce,
            );
            return;
        }
    }

    emit_repo_list(state, ch, session, switch_nonce);
    if let Some(fallback) = fallback {
        super::switcher_repo::handle_switch_repo(
            state,
            ch,
            session,
            fallback.name,
            Some(fallback.repo_id),
            switch_nonce,
        )
        .await;
    }
}

fn active_repo_id(state: &Arc<AppState>, session: &WsSession) -> Option<RepoId> {
    if let Some(repo_id) = session.active_repo_id {
        return Some(repo_id);
    }
    session.active_repo.as_deref().and_then(|name| {
        state
            .repo
            .get_repo_info_for(None, Some(name))
            .ok()
            .flatten()
            .map(|info| info.uuid)
    })
}

fn emit_repo_list(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    switch_nonce: Option<u64>,
) {
    match repo_list_message(
        state,
        None,
        None,
        session.is_browser_session().then(|| session.scope_nonce()),
    ) {
        Ok(message) => ch.unicast(message),
        Err(err) => ch.send_protocol_error_with_switch_nonce(
            invalid_repo_context(format!("Repository changed but list refresh failed: {err}")),
            switch_nonce,
        ),
    }
}

fn invalid_repo_context(detail: impl Into<String>) -> ServerError {
    ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, detail)
}

#[cfg(test)]
mod tests {
    use super::{handle_remove_repo, handle_rename_repo};
    use crate::server::switcher_test_support::{browser_session, build_state, unicast_channel};
    use deve_core::protocol::{ServerErrorCode, ServerMessage};
    use tokio::sync::mpsc;
    use tokio::time::{Duration, timeout};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rename_repo_updates_repo_list_entry() -> anyhow::Result<()> {
        let (_dir, state) = build_state()?;
        let (ch, mut rx) = unicast_channel(&state);
        let mut session = browser_session(3);
        let repo_id = state
            .repo
            .get_repo_info_for(None, Some("default"))?
            .expect("default repo")
            .uuid;
        state.sync_manager.materialize_local_repo("default")?;
        session.active_repo = Some("default".to_string());
        session.active_repo_id = Some(repo_id);

        handle_rename_repo(
            &state,
            &ch,
            &mut session,
            repo_id,
            "renamed".into(),
            Some(4),
        )
        .await;

        match recv(&mut rx).await? {
            ServerMessage::RepoList { repo_entries, .. } => {
                assert!(
                    repo_entries
                        .iter()
                        .any(|entry| { entry.repo_id == repo_id && entry.name == "renamed" })
                );
            }
            other => anyhow::bail!("expected RepoList, got {other:?}"),
        }
        match recv_until_repo_switched(&mut rx).await? {
            ServerMessage::RepoSwitched { name, uuid, .. } => {
                assert_eq!(name, "renamed");
                assert_eq!(uuid::Uuid::parse_str(&uuid)?, repo_id);
            }
            other => anyhow::bail!("expected RepoSwitched, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remove_repo_hides_it_from_repo_list_without_deleting_authority() -> anyhow::Result<()>
    {
        let (_dir, state) = build_state()?;
        let (ch, mut rx) = unicast_channel(&state);
        let mut session = browser_session(8);

        super::super::switcher_create_repo::handle_create_repo(
            &state,
            &ch,
            &mut session,
            "research".into(),
            Some(9),
        )
        .await;
        let research_id = recv_until_repo_switched(&mut rx).await?;
        let research_id = match research_id {
            ServerMessage::RepoSwitched { uuid, .. } => uuid::Uuid::parse_str(&uuid)?,
            other => anyhow::bail!("expected RepoSwitched, got {other:?}"),
        };

        handle_remove_repo(&state, &ch, &mut session, research_id, Some(10)).await;

        let repo_list = recv_until_repo_list(&mut rx).await?;
        match repo_list {
            ServerMessage::RepoList {
                repos,
                repo_entries,
                ..
            } => {
                assert!(!repos.iter().any(|repo| repo == "research"));
                assert!(
                    !repo_entries
                        .iter()
                        .any(|entry| entry.repo_id == research_id)
                );
            }
            other => anyhow::bail!("expected RepoList, got {other:?}"),
        }
        match recv_until_repo_switched(&mut rx).await? {
            ServerMessage::RepoSwitched { name, uuid, .. } => {
                assert_eq!(name, "default");
                assert_ne!(uuid::Uuid::parse_str(&uuid)?, research_id);
            }
            other => anyhow::bail!("expected fallback RepoSwitched, got {other:?}"),
        }
        assert_eq!(session.active_repo.as_deref(), Some("default"));
        assert!(state.repo.get_local_repo_info_by_id(research_id)?.is_none());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remove_last_repo_is_rejected() -> anyhow::Result<()> {
        let (_dir, state) = build_state()?;
        let (ch, mut rx) = unicast_channel(&state);
        let mut session = browser_session(1);
        let repo_id = state
            .repo
            .get_repo_info_for(None, Some("default"))?
            .expect("default repo")
            .uuid;
        session.active_repo = Some("default".to_string());
        session.active_repo_id = Some(repo_id);

        handle_remove_repo(&state, &ch, &mut session, repo_id, Some(2)).await;

        match recv(&mut rx).await? {
            ServerMessage::ProtocolError { error, .. } => {
                assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            }
            other => anyhow::bail!("expected ProtocolError, got {other:?}"),
        }
        Ok(())
    }

    async fn recv(rx: &mut mpsc::Receiver<ServerMessage>) -> anyhow::Result<ServerMessage> {
        timeout(Duration::from_secs(2), rx.recv())
            .await?
            .ok_or_else(|| anyhow::anyhow!("channel closed"))
    }

    async fn recv_until_repo_switched(
        rx: &mut mpsc::Receiver<ServerMessage>,
    ) -> anyhow::Result<ServerMessage> {
        for _ in 0..6 {
            let message = recv(rx).await?;
            if matches!(message, ServerMessage::RepoSwitched { .. }) {
                return Ok(message);
            }
        }
        anyhow::bail!("RepoSwitched not received")
    }

    async fn recv_until_repo_list(
        rx: &mut mpsc::Receiver<ServerMessage>,
    ) -> anyhow::Result<ServerMessage> {
        for _ in 0..6 {
            let message = recv(rx).await?;
            if matches!(message, ServerMessage::RepoList { .. }) {
                return Ok(message);
            }
        }
        anyhow::bail!("RepoList not received")
    }
}
