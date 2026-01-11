//! # Search Handler (搜索处理器)
//!
//! **架构作用**:
//! 处理来自客户端的搜索请求。
//!
//! **核心功能清单**:
//! - `handle_search`: 执行全文搜索并返回结果。
//!
//! **类型**: Plugin MAY (插件可选) - 仅 Standard Profile 启用

use std::sync::Arc;
use tokio::sync::broadcast;
use deve_core::protocol::ServerMessage;
use crate::server::AppState;

#[cfg(feature = "search")]
pub async fn handle_search(
    state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    query: String,
    limit: u32,
) {
    if let Some(ref search_service) = state.search_service {
        match search_service.search(&query, limit as usize) {
            Ok(results) => {
                let results: Vec<(String, String, f32)> = results
                    .into_iter()
                    .map(|r| (r.doc_id, r.path, r.score))
                    .collect();
                let _ = tx.send(ServerMessage::SearchResults { results });
            }
            Err(e) => {
                let _ = tx.send(ServerMessage::Error(format!("Search failed: {}", e)));
            }
        }
    } else {
        let _ = tx.send(ServerMessage::Error("Search feature not enabled".to_string()));
    }
}

#[cfg(not(feature = "search"))]
pub async fn handle_search(
    _state: &Arc<AppState>,
    tx: &broadcast::Sender<ServerMessage>,
    _query: String,
    _limit: u32,
) {
    let _ = tx.send(ServerMessage::Error("Search feature not enabled".to_string()));
}
