// apps/cli/src/server/handlers/search.rs
//! # 搜索处理器 (Search Handler)
//!
//! 处理来自客户端的搜索请求

use crate::server::AppState;
use crate::server::channel::DualChannel;
#[cfg(feature = "search")]
use crate::server::error_classify::is_storage_corruption;
#[cfg(feature = "search")]
use deve_core::protocol::ServerMessage;
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

#[cfg(feature = "search")]
pub async fn handle_search(
    state: &Arc<AppState>,
    ch: &DualChannel,
    request_id: String,
    query: String,
    limit: u32,
    scope_nonce: Option<u64>,
) {
    if let Some(ref search_service) = state.search_service {
        match search_service.search(&query, limit as usize) {
            Ok(results) => {
                let results: Vec<(String, String, f32)> = results
                    .into_iter()
                    .map(|r| (r.doc_id, r.path, r.score))
                    .collect();
                // 单播搜索结果给请求者
                ch.unicast(ServerMessage::SearchResults {
                    request_id,
                    scope_nonce,
                    results,
                });
            }
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
    } else {
        ch.send_protocol_error_with_scope_nonce(
            ServerError::with_detail(ServerErrorCode::RequestFailed, "Search feature not enabled"),
            scope_nonce,
        );
    }
}

#[cfg(not(feature = "search"))]
pub async fn handle_search(
    _state: &Arc<AppState>,
    ch: &DualChannel,
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
#[cfg_attr(not(test), allow(dead_code))]
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
    use super::classify_search_error;
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
}
