use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::run_on_resolved_local_repo;
use crate::server::session::WsSession;
use deve_core::protocol::ServerMessage;
use std::collections::HashSet;
use std::sync::Arc;

/// 暂存指定文件
pub async fn handle_stage_file(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    path: String,
) {
    let path = deve_core::utils::path::to_forward_slash(&path);
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return ch.send_error(e.to_string()),
    };
    match state.repo.stage_file_in_local_repo(&scope.repo_name, &path) {
        Ok(()) => {
            tracing::info!("Staged file: {}", path);
            ch.unicast(ServerMessage::StageAck { path });
        }
        Err(e) => {
            tracing::error!("Failed to stage file: {:?}", e);
            ch.send_error(e.to_string());
        }
    }
}

/// 取消暂存指定文件
pub async fn handle_unstage_file(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    path: String,
) {
    let path = deve_core::utils::path::to_forward_slash(&path);
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return ch.send_error(e.to_string()),
    };
    match run_on_resolved_local_repo(state, &scope, |db| {
        deve_core::ledger::source_control::unstage_file(db, &path)
    }) {
        Ok(()) => {
            tracing::info!("Unstaged file: {}", path);
            ch.unicast(ServerMessage::UnstageAck { path });
        }
        Err(e) => {
            tracing::error!("Failed to unstage file: {:?}", e);
            ch.send_error(e.to_string());
        }
    }
}

/// 批量暂存文件
pub async fn handle_stage_files(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    paths: Vec<String>,
) {
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return ch.send_error(e.to_string()),
    };
    let paths = normalized_unique_paths(paths);
    for path in &paths {
        if let Err(e) = state.repo.stage_file_in_local_repo(&scope.repo_name, path) {
            tracing::error!("Failed to stage files: {:?}", e);
            ch.send_error(e.to_string());
            return;
        }
    }
    super::changes::handle_get_changes(state, ch, session).await;
}

/// 批量取消暂存文件
pub async fn handle_unstage_files(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    paths: Vec<String>,
) {
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return ch.send_error(e.to_string()),
    };
    let paths = normalized_unique_paths(paths);
    if let Err(e) = run_on_resolved_local_repo(state, &scope, |db| {
        for path in &paths {
            deve_core::ledger::source_control::unstage_file(db, path)?;
        }
        Ok(())
    }) {
        tracing::error!("Failed to unstage files: {:?}", e);
        ch.send_error(e.to_string());
        return;
    }
    super::changes::handle_get_changes(state, ch, session).await;
}

fn normalized_unique_paths(paths: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .map(|p| deve_core::utils::path::to_forward_slash(&p))
        .filter(|p| !p.is_empty())
        .filter(|p| seen.insert(p.clone()))
        .collect()
}
