// apps/web/src/hooks/use_core/state.rs
//! # 状态信号声明
//!
//! 定义 `use_core` 所需的所有响应式信号。

use crate::editor::EditorStats;
use deve_core::models::{DocId, PeerId};
use deve_core::source_control::{ChangeEntry, CommitInfo};
use deve_core::tree::FileNode;
use leptos::prelude::*;
use std::collections::HashMap;

use super::types::PeerSession;

/// 核心状态信号集合
///
/// 包含所有 `use_core` 需要的响应式信号。
pub struct CoreSignals {
    // 文档状态
    pub docs: ReadSignal<Vec<(DocId, String)>>,
    pub set_docs: WriteSignal<Vec<(DocId, String)>>,
    pub current_doc: ReadSignal<Option<DocId>>,
    pub set_current_doc: WriteSignal<Option<DocId>>,

    // 编辑器统计
    pub stats: ReadSignal<EditorStats>,
    pub set_stats: WriteSignal<EditorStats>,

    // P2P 状态
    pub peers: ReadSignal<HashMap<PeerId, PeerSession>>,
    pub set_peers: WriteSignal<HashMap<PeerId, PeerSession>>,

    // 插件
    pub plugin_response: ReadSignal<Option<(String, Option<serde_json::Value>, Option<String>)>>,
    pub set_plugin_response:
        WriteSignal<Option<(String, Option<serde_json::Value>, Option<String>)>>,

    // 搜索
    pub search_results: ReadSignal<Vec<(String, String, f32)>>,
    pub set_search_results: WriteSignal<Vec<(String, String, f32)>>,

    // 手动合并
    pub sync_mode: ReadSignal<String>,
    pub set_sync_mode: WriteSignal<String>,
    pub pending_ops_count: ReadSignal<u32>,
    pub set_pending_ops_count: WriteSignal<u32>,
    pub pending_ops_previews: ReadSignal<Vec<(String, String, String)>>,
    pub set_pending_ops_previews: WriteSignal<Vec<(String, String, String)>>,

    // 分支/仓库
    pub active_repo: ReadSignal<Option<PeerId>>,
    pub set_active_repo: WriteSignal<Option<PeerId>>,
    pub shadow_repos: ReadSignal<Vec<String>>,
    pub set_shadow_repos: WriteSignal<Vec<String>>,
    pub repo_list: ReadSignal<Vec<String>>,
    pub set_repo_list: WriteSignal<Vec<String>>,

    // 版本/回放
    pub doc_version: ReadSignal<u64>,
    pub set_doc_version: WriteSignal<u64>,
    pub playback_version: ReadSignal<u64>,
    pub set_playback_version: WriteSignal<u64>,
    pub is_spectator: Memo<bool>,

    // Source Control
    pub staged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub set_staged_changes: WriteSignal<Vec<ChangeEntry>>,
    pub unstaged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub set_unstaged_changes: WriteSignal<Vec<ChangeEntry>>,
    pub commit_history: ReadSignal<Vec<CommitInfo>>,
    pub set_commit_history: WriteSignal<Vec<CommitInfo>>,
    pub diff_content: ReadSignal<Option<(String, String, String)>>,
    pub set_diff_content: WriteSignal<Option<(String, String, String)>>,

    // 文件树 (增量更新)
    pub tree_nodes: ReadSignal<Vec<FileNode>>,
    pub set_tree_nodes: WriteSignal<Vec<FileNode>>,
}

/// 初始化所有核心信号
pub fn init_signals() -> CoreSignals {
    let (docs, set_docs) = signal(Vec::<(DocId, String)>::new());
    let (current_doc, set_current_doc) = signal(None::<DocId>);
    let (stats, set_stats) = signal(EditorStats::default());
    let (peers, set_peers) = signal(HashMap::<PeerId, PeerSession>::new());
    let (plugin_response, set_plugin_response) = signal(None);
    let (search_results, set_search_results) = signal(Vec::new());
    let (sync_mode, set_sync_mode) = signal("auto".to_string());
    let (pending_ops_count, set_pending_ops_count) = signal(0u32);
    let (pending_ops_previews, set_pending_ops_previews) = signal(Vec::new());
    let (active_repo, set_active_repo) = signal(None::<PeerId>);
    let (shadow_repos, set_shadow_repos) = signal(Vec::new());
    let (repo_list, set_repo_list) = signal(Vec::new());
    let (doc_version, set_doc_version) = signal(0u64);
    let (playback_version, set_playback_version) = signal(0u64);
    let is_spectator = Memo::new(move |_| active_repo.get().is_some());
    let (staged_changes, set_staged_changes) = signal(Vec::new());
    let (unstaged_changes, set_unstaged_changes) = signal(Vec::new());
    let (commit_history, set_commit_history) = signal(Vec::new());
    let (diff_content, set_diff_content) = signal(None);
    let (tree_nodes, set_tree_nodes) = signal(Vec::<FileNode>::new());

    CoreSignals {
        docs,
        set_docs,
        current_doc,
        set_current_doc,
        stats,
        set_stats,
        peers,
        set_peers,
        plugin_response,
        set_plugin_response,
        search_results,
        set_search_results,
        sync_mode,
        set_sync_mode,
        pending_ops_count,
        set_pending_ops_count,
        pending_ops_previews,
        set_pending_ops_previews,
        active_repo,
        set_active_repo,
        shadow_repos,
        set_shadow_repos,
        repo_list,
        set_repo_list,
        doc_version,
        set_doc_version,
        playback_version,
        set_playback_version,
        is_spectator,
        staged_changes,
        set_staged_changes,
        unstaged_changes,
        set_unstaged_changes,
        commit_history,
        set_commit_history,
        diff_content,
        set_diff_content,
        tree_nodes,
        set_tree_nodes,
    }
}
