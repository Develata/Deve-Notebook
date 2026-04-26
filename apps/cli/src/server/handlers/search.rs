// apps/cli/src/server/handlers/search.rs
//! # 搜索处理器 (Search Handler)
//!
//! 处理来自客户端的搜索请求

use crate::server::AppState;
use crate::server::channel::DualChannel;
#[cfg(feature = "search")]
use crate::server::error_classify::is_storage_corruption;
#[cfg(feature = "search")]
use crate::server::repo_scope::{
    ResolvedRepo, map_repo_scope_error, resolve_session_repo_or_bootstrap_local,
};
use crate::server::session::WsSession;
#[cfg(feature = "search")]
use deve_core::ledger::listing::RepoListing;
#[cfg(feature = "search")]
use deve_core::models::{DocId, RepoType};
#[cfg(feature = "search")]
use deve_core::protocol::ServerMessage;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

#[cfg(feature = "search")]
pub async fn handle_search(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
    query: String,
    limit: u32,
    scope_nonce: Option<u64>,
) {
    if state.search_service.is_none() {
        ch.send_protocol_error_with_scope_nonce(
            ServerError::with_detail(
                ServerErrorCode::RequestFailed,
                "Search feature disabled for current runtime profile",
            ),
            scope_nonce,
        );
        return;
    }

    let scope = match resolve_session_repo_or_bootstrap_local(state, session) {
        Ok(scope) => scope,
        Err(err) => {
            ch.send_protocol_error_with_scope_nonce(map_repo_scope_error(err), scope_nonce);
            return;
        }
    };
    if scope.branch.is_none()
        && (session.active_repo.as_deref() != Some(scope.repo_name.as_str())
            || session.active_repo_id != Some(scope.repo_id))
    {
        session.switch_repo(scope.repo_name.clone(), Some(scope.repo_id));
    }

    match search_scope_documents(state, &scope, &query, limit as usize) {
        Ok(results) => ch.unicast(ServerMessage::SearchResults {
            request_id,
            repo_id: Some(scope.repo_id),
            branch: scope.branch.clone(),
            scope_nonce,
            results,
        }),
        Err(e) => {
            ch.send_protocol_error_with_scope_nonce(
                ServerError::with_detail(
                    classify_search_error(&e.to_string()),
                    format!("Search failed: {}", e),
                ),
                scope_nonce,
            );
        }
    }
}

#[cfg(not(feature = "search"))]
pub async fn handle_search(
    _state: &Arc<AppState>,
    ch: &DualChannel,
    _session: &mut WsSession,
    _request_id: String,
    _query: String,
    _limit: u32,
    scope_nonce: Option<u64>,
) {
    ch.send_protocol_error_with_scope_nonce(
        ServerError::with_detail(ServerErrorCode::RequestFailed, "Search feature not enabled"),
        scope_nonce,
    );
}

#[cfg(feature = "search")]
fn search_scope_documents(
    state: &Arc<AppState>,
    scope: &ResolvedRepo,
    query: &str,
    limit: usize,
) -> anyhow::Result<Vec<(String, String, f32)>> {
    let normalized_query = query.trim().to_lowercase();
    if normalized_query.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let repo_type = repo_type_for_scope(scope);
    let docs = state.repo.list_docs(&repo_type)?;
    let mut results = Vec::new();
    for (doc_id, path) in docs {
        let content = reconstruct_doc_content(state, &repo_type, doc_id)?;
        if let Some(score) = score_match(&path, &content, &normalized_query) {
            results.push((doc_id.to_string(), path, score));
        }
    }
    results.sort_by(|a, b| {
        b.2.partial_cmp(&a.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
    });
    results.truncate(limit);
    Ok(results)
}

#[cfg(feature = "search")]
fn repo_type_for_scope(scope: &ResolvedRepo) -> RepoType {
    match &scope.branch {
        Some(peer_id) => RepoType::Remote(peer_id.clone(), scope.repo_id),
        None => RepoType::Local(scope.repo_id),
    }
}

#[cfg(feature = "search")]
fn reconstruct_doc_content(
    state: &Arc<AppState>,
    repo_type: &RepoType,
    doc_id: DocId,
) -> anyhow::Result<String> {
    let ops = state.repo.get_ops(repo_type, doc_id)?;
    let entries = ops.into_iter().map(|(_, entry)| entry).collect::<Vec<_>>();
    Ok(deve_core::state::reconstruct_content(&entries))
}

#[cfg(feature = "search")]
fn score_match(path: &str, content: &str, query: &str) -> Option<f32> {
    let path_lower = path.to_lowercase();
    let content_lower = content.to_lowercase();
    let mut score = 0.0;
    if path_lower.contains(query) {
        score += 2.0;
    }
    if content_lower.contains(query) {
        score += 1.0;
    }
    (score > 0.0).then_some(score)
}

#[cfg(feature = "search")]
fn classify_search_error(detail: &str) -> ServerErrorCode {
    let lower = detail.to_lowercase();
    if is_storage_corruption(&lower)
        || lower.contains("search index document missing required stored field")
        || lower.contains("searchservice writer lock poisoned")
    {
        return ServerErrorCode::StoragePersistFailed;
    }
    ServerErrorCode::RequestFailed
}

#[cfg(all(test, feature = "search"))]
mod tests {
    use super::{classify_search_error, score_match, search_scope_documents};
    use crate::server::edit_state_test_support::{edit_harness, seed_doc_with_content};
    use crate::server::repo_scope::ResolvedRepo;
    use deve_core::protocol::ServerErrorCode;

    #[test]
    fn classifies_search_index_corruption_as_storage_persist_failed() {
        assert_eq!(
            classify_search_error("Search index document missing required stored field: path"),
            ServerErrorCode::StoragePersistFailed
        );
        assert_eq!(
            classify_search_error("SearchService writer lock poisoned"),
            ServerErrorCode::StoragePersistFailed
        );
    }

    #[test]
    fn keeps_user_query_errors_as_request_failed() {
        assert_eq!(
            classify_search_error("The query parser expected a term"),
            ServerErrorCode::RequestFailed
        );
    }

    #[test]
    fn scores_path_matches_above_content_matches() {
        assert!(score_match("notes/rust.md", "plain text", "rust").unwrap() > 1.0);
        assert_eq!(score_match("notes/a.md", "Rust content", "rust"), Some(1.0));
        assert_eq!(score_match("notes/a.md", "plain text", "rust"), None);
    }

    #[test]
    fn scope_search_scans_current_repo_documents() -> anyhow::Result<()> {
        let h = edit_harness(true)?;
        let rust_doc = seed_doc_with_content(&h.state, "default", "notes/rust.md", "Rust search")?;
        seed_doc_with_content(&h.state, "test", "notes/rust.md", "Other repo Rust")?;
        let scope = ResolvedRepo {
            repo_id: h.default_repo_id,
            repo_name: "default".into(),
            branch: None,
        };

        let results = search_scope_documents(&h.state, &scope, "rust", 10)?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, rust_doc.to_string());
        assert_eq!(results[0].1, "notes/rust.md");
        Ok(())
    }

    #[test]
    fn scope_search_honors_limit_and_blank_query() -> anyhow::Result<()> {
        let h = edit_harness(false)?;
        seed_doc_with_content(&h.state, "default", "notes/a.md", "needle")?;
        seed_doc_with_content(&h.state, "default", "notes/b.md", "needle")?;
        let scope = ResolvedRepo {
            repo_id: h.default_repo_id,
            repo_name: "default".into(),
            branch: None,
        };

        assert!(search_scope_documents(&h.state, &scope, "   ", 10)?.is_empty());
        assert_eq!(
            search_scope_documents(&h.state, &scope, "needle", 1)?.len(),
            1
        );
        Ok(())
    }
}
