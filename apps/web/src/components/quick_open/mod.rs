// apps\web\src\components\quick_open
//! # Quick Open Module (快速打开模块)
//!
//! 提供文件搜索功能，支持模糊匹配和 MRU (Most Recently Used) 列表。
//! 符合 `03_ui_architecture.md` 规范。

use crate::components::search_box::types::{SearchProvider, SearchResult, SearchAction};
use deve_core::models::DocId;

/// 文件搜索 Provider
pub struct FileSearchProvider {
    docs: Vec<(DocId, String)>,
}

impl FileSearchProvider {
    pub fn new(docs: Vec<(DocId, String)>) -> Self {
        Self { docs }
    }
}

impl SearchProvider for FileSearchProvider {
    fn trigger_char(&self) -> Option<char> {
        None // 默认模式，无触发符
    }

    fn search(&self, query: &str) -> Vec<SearchResult> {
        if query.is_empty() {
             return self.docs.iter().take(20).map(|(id, path)| {
                SearchResult {
                    id: id.to_string(),
                    title: path.clone(),
                    detail: None,
                    score: 1.0,
                    action: SearchAction::OpenDoc(*id),
                }
            }).collect();
        }

        let mut results: Vec<SearchResult> = self.docs.iter()
            .map(|(id, path)| {
                let score = sublime_fuzzy::best_match(query, path)
                    .map(|m| m.score() as f32)
                    .unwrap_or(0.0);
                (id, path, score)
            })
            .filter(|(_, _, score)| *score > 0.0)
            .map(|(id, path, score)| {
                SearchResult {
                    id: id.to_string(),
                    title: path.clone(),
                    detail: None,
                    score,
                    action: SearchAction::OpenDoc(*id),
                }
            })
            .collect();
        
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(20);
        results
    }

    fn execute(&self, _action: &SearchAction) {
        // 执行由组件处理
    }
}
