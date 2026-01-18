// apps/cli/src/server/handlers/search.rs
//! # 搜索处理器 (Search Handler)
//!
//! 处理来自客户端的搜索请求

use crate::server::AppState;
use crate::server::channel::DualChannel;
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

#[cfg(feature = "search")]
pub async fn handle_search(state: &Arc<AppState>, ch: &DualChannel, query: String, limit: u32) {
    if let Some(ref search_service) = state.search_service {
        match search_service.search(&query, limit as usize) {
            Ok(results) => {
                let results: Vec<(String, String, f32)> = results
                    .into_iter()
                    .map(|r| (r.doc_id, r.path, r.score))
                    .collect();
                // 单播搜索结果给请求者
                ch.unicast(ServerMessage::SearchResults { results });
            }
            Err(e) => {
                ch.send_error(format!("Search failed: {}", e));
            }
        }
    } else {
        ch.send_error("Search feature not enabled".to_string());
    }
}

#[cfg(not(feature = "search"))]
pub async fn handle_search(_state: &Arc<AppState>, ch: &DualChannel, _query: String, _limit: u32) {
    ch.send_error("Search feature not enabled".to_string());
}
