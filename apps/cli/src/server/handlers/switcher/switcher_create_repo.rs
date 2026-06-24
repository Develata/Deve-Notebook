//! plan_ref:
//!   - 03_storage/projection#projection-locator-contract
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-scope-runtime
//!
//! Local repo creation handler for browser-driven repo switching.

use crate::repo_init::initialize_local_repo_workspace;
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::repo_list::repo_list_message;
use crate::server::session::WsSession;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::path::PathBuf;
use std::sync::Arc;

pub(super) async fn handle_create_repo(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    raw_name: String,
    switch_nonce: Option<u64>,
) {
    if !super::switcher_guard::require_browser_switch_nonce(
        ch,
        session,
        switch_nonce,
        "repo create",
    ) {
        return;
    }
    if session.active_branch.is_some() {
        ch.send_protocol_error_with_switch_nonce(
            invalid_repo_context("Cannot create a local repository while viewing a remote branch"),
            switch_nonce,
        );
        return;
    }
    let repo_name = raw_name.trim().to_string();
    if repo_name.is_empty() {
        ch.send_protocol_error_with_switch_nonce(
            invalid_repo_context("Repository name must not be empty"),
            switch_nonce,
        );
        return;
    }
    match state.repo.get_repo_info_for(None, Some(&repo_name)) {
        Ok(Some(_)) => {
            ch.send_protocol_error_with_switch_nonce(
                invalid_repo_context(format!("Repository already exists: {repo_name}")),
                switch_nonce,
            );
            return;
        }
        Ok(None) => {}
        Err(err) => {
            ch.send_protocol_error_with_switch_nonce(
                invalid_repo_context(format!("Failed to check repository name: {err}")),
                switch_nonce,
            );
            return;
        }
    }

    let projection_base = match projection_base_for_new_repo(state, session) {
        Ok(base) => base,
        Err(err) => {
            ch.send_protocol_error_with_switch_nonce(
                ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, err.to_string()),
                switch_nonce,
            );
            return;
        }
    };
    let report = match initialize_local_repo_workspace(
        state.repo.ledger_dir(),
        &repo_name,
        &projection_base,
        state.repo.snapshot_depth(),
        None,
        None,
    ) {
        Ok(report) => report,
        Err(err) => {
            ch.send_protocol_error_with_switch_nonce(
                ServerError::with_detail(
                    ServerErrorCode::StoragePersistFailed,
                    format!("Failed to create repository {repo_name}: {err}"),
                ),
                switch_nonce,
            );
            return;
        }
    };
    tracing::info!(
        repo_name = %report.repo_name,
        repo_id = %report.repo_id,
        workspace_root = ?report.workspace_root,
        "Created local repository from browser session"
    );

    match repo_list_message(
        state,
        None,
        None,
        session.is_browser_session().then(|| session.scope_nonce()),
    ) {
        Ok(message) => ch.unicast(message),
        Err(err) => {
            ch.send_protocol_error_with_switch_nonce(
                invalid_repo_context(format!("Repository created but list refresh failed: {err}")),
                switch_nonce,
            );
            return;
        }
    }

    super::switcher_repo::handle_switch_repo(
        state,
        ch,
        session,
        report.repo_name,
        Some(report.repo_id),
        switch_nonce,
    )
    .await;
}

fn projection_base_for_new_repo(
    state: &Arc<AppState>,
    session: &WsSession,
) -> anyhow::Result<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(repo) = session.active_repo.as_deref() {
        candidates.push(repo);
    }
    if let Some(repo) = session.last_local_repo.as_deref() {
        candidates.push(repo);
    }
    candidates.push(state.repo.local_repo_name());

    for repo_name in candidates {
        if let Ok(locator) = state.repo.projection_locator_for_local_repo(repo_name) {
            return Ok(locator.projection_base_abs);
        }
    }
    Err(anyhow::anyhow!(
        "Projection Locator missing for current local repository"
    ))
}

fn invalid_repo_context(detail: impl Into<String>) -> ServerError {
    ServerError::with_detail(ServerErrorCode::ScRepoContextInvalid, detail)
}

#[cfg(test)]
mod tests {
    use super::handle_create_repo;
    use crate::server::switcher_test_support::{browser_session, build_state, unicast_channel};
    use deve_core::protocol::{ServerErrorCode, ServerMessage};
    use tokio::sync::mpsc;
    use tokio::time::{Duration, timeout};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn create_repo_initializes_workspace_and_switches_session() -> anyhow::Result<()> {
        let (dir, state) = build_state()?;
        let (ch, mut rx) = unicast_channel(&state);
        let mut session = browser_session(3);

        handle_create_repo(&state, &ch, &mut session, "research".into(), Some(4)).await;

        let repo_list = recv(&mut rx).await?;
        match repo_list {
            ServerMessage::RepoList {
                repos, scope_nonce, ..
            } => {
                assert_eq!(scope_nonce, Some(3));
                assert!(repos.iter().any(|repo| repo == "default"));
                assert!(repos.iter().any(|repo| repo == "research"));
            }
            other => anyhow::bail!("expected RepoList, got {other:?}"),
        }

        let switched = recv_until_repo_switched(&mut rx).await?;
        match switched {
            ServerMessage::RepoSwitched {
                name,
                uuid,
                switch_nonce,
                ..
            } => {
                assert_eq!(name, "research");
                assert_eq!(switch_nonce, Some(4));
                uuid::Uuid::parse_str(&uuid)?;
            }
            other => anyhow::bail!("expected RepoSwitched, got {other:?}"),
        }
        assert_eq!(session.active_repo.as_deref(), Some("research"));
        assert_eq!(session.scope_nonce(), 4);
        let workspace_exists = std::fs::read_dir(dir.path().join("notes"))?.any(|entry| {
            entry
                .ok()
                .and_then(|entry| entry.file_name().into_string().ok())
                .is_some_and(|name| name.starts_with("research--"))
        });
        assert!(workspace_exists);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn create_repo_rejects_duplicate_name() -> anyhow::Result<()> {
        let (_dir, state) = build_state()?;
        let (ch, mut rx) = unicast_channel(&state);
        let mut session = browser_session(9);

        handle_create_repo(&state, &ch, &mut session, "default".into(), Some(10)).await;

        match recv(&mut rx).await? {
            ServerMessage::ProtocolError {
                error,
                switch_nonce,
                ..
            } => {
                assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
                assert_eq!(switch_nonce, Some(10));
                assert!(
                    error
                        .detail
                        .as_deref()
                        .is_some_and(|detail| detail.contains("already exists"))
                );
            }
            other => anyhow::bail!("expected ProtocolError, got {other:?}"),
        }
        assert!(session.active_repo.is_none());
        assert_eq!(session.scope_nonce(), 9);
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
        for _ in 0..4 {
            let message = recv(rx).await?;
            if matches!(message, ServerMessage::RepoSwitched { .. }) {
                return Ok(message);
            }
        }
        anyhow::bail!("RepoSwitched not received")
    }
}
