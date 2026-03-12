// apps/web/src/hooks/use_core/state.rs
//! # 状态信号声明
//!
//! 定义 `use_core` 所需的所有响应式信号。

use crate::editor::EditorStats;
use crate::storage::DegradedSyncMode;
use deve_core::models::{DocId, PeerId};
use deve_core::source_control::{ChangeEntry, CommitFileDiff, CommitInfo};
use deve_core::tree::FileNode;
use leptos::prelude::*;
use std::collections::HashMap;

use super::contexts::SystemMetricsData;
use super::diff_session::DiffSessionWire;
use super::navigation::PendingNavigation;
use super::pending::PendingLocalEdits;
use super::types::{ChatMessage, PeerSession, PendingBranchTarget};

pub use super::state_init::init_signals;

/// 插件响应类型别名
pub type PluginResponse = Option<(
    String,
    Option<serde_json::Value>,
    Option<deve_core::protocol::ServerError>,
)>;

/// 核心状态信号集合
///
/// 包含所有 `use_core` 需要的响应式信号。
#[derive(Clone, Copy)]
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
    pub handshake_ready: ReadSignal<bool>,
    pub set_handshake_ready: WriteSignal<bool>,
    pub pending_local_edits: ReadSignal<PendingLocalEdits>,
    pub set_pending_local_edits: WriteSignal<PendingLocalEdits>,
    pub pending_navigation: ReadSignal<Option<PendingNavigation>>,
    pub set_pending_navigation: WriteSignal<Option<PendingNavigation>>,

    // 插件
    pub plugin_response: ReadSignal<PluginResponse>,
    pub set_plugin_response: WriteSignal<PluginResponse>,
    pub plugin_request_ids: ReadSignal<Vec<String>>,
    pub set_plugin_request_ids: WriteSignal<Vec<String>>,

    // AI Chat
    pub chat_messages: ReadSignal<Vec<ChatMessage>>,
    pub set_chat_messages: WriteSignal<Vec<ChatMessage>>,
    pub is_chat_streaming: ReadSignal<bool>,
    pub set_is_chat_streaming: WriteSignal<bool>,
    pub ai_mode: ReadSignal<String>,
    pub set_ai_mode: WriteSignal<String>,

    // 搜索
    pub search_request_id: ReadSignal<Option<String>>,
    pub set_search_request_id: WriteSignal<Option<String>>,
    pub search_results: ReadSignal<Vec<(String, String, f32)>>,
    pub set_search_results: WriteSignal<Vec<(String, String, f32)>>,

    // 文档加载状态
    pub load_state: ReadSignal<String>,
    pub set_load_state: WriteSignal<String>,
    pub load_progress: ReadSignal<(usize, usize)>,
    pub set_load_progress: WriteSignal<(usize, usize)>,
    pub load_eta_ms: ReadSignal<u64>,
    pub set_load_eta_ms: WriteSignal<u64>,

    // 手动合并
    pub sync_mode: ReadSignal<String>,
    pub set_sync_mode: WriteSignal<String>,
    pub pending_ops_count: ReadSignal<u32>,
    pub set_pending_ops_count: WriteSignal<u32>,
    pub pending_ops_previews: ReadSignal<Vec<(String, String, String)>>,
    pub set_pending_ops_previews: WriteSignal<Vec<(String, String, String)>>,

    // 分支/仓库
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub set_active_branch: WriteSignal<Option<PeerId>>,
    pub pending_branch_switch: ReadSignal<Option<PendingBranchTarget>>,
    pub set_pending_branch_switch: WriteSignal<Option<PendingBranchTarget>>,
    pub current_repo: ReadSignal<Option<String>>,
    pub set_current_repo: WriteSignal<Option<String>>,
    pub current_repo_id: ReadSignal<Option<String>>,
    pub set_current_repo_id: WriteSignal<Option<String>>,
    pub pending_repo_switch: ReadSignal<Option<String>>,
    pub set_pending_repo_switch: WriteSignal<Option<String>>,
    pub shadow_repos: ReadSignal<Vec<String>>,
    pub set_shadow_repos: WriteSignal<Vec<String>>,
    pub shadow_list_request_id: ReadSignal<Option<String>>,
    pub set_shadow_list_request_id: WriteSignal<Option<String>>,
    pub repo_list: ReadSignal<Vec<String>>,
    pub set_repo_list: WriteSignal<Vec<String>>,
    pub repo_list_request_id: ReadSignal<Option<String>>,
    pub set_repo_list_request_id: WriteSignal<Option<String>>,
    pub doc_list_request_id: ReadSignal<Option<String>>,
    pub set_doc_list_request_id: WriteSignal<Option<String>>,
    pub tree_request_id: ReadSignal<Option<String>>,
    pub set_tree_request_id: WriteSignal<Option<String>>,

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
    pub changes_request_id: ReadSignal<Option<String>>,
    pub set_changes_request_id: WriteSignal<Option<String>>,
    pub commit_history: ReadSignal<Vec<CommitInfo>>,
    pub set_commit_history: WriteSignal<Vec<CommitInfo>>,
    pub commit_history_request_id: ReadSignal<Option<String>>,
    pub set_commit_history_request_id: WriteSignal<Option<String>>,
    pub doc_diff_request_id: ReadSignal<Option<String>>,
    pub set_doc_diff_request_id: WriteSignal<Option<String>>,
    pub diff_content: ReadSignal<Option<DiffSessionWire>>,
    pub set_diff_content: WriteSignal<Option<DiffSessionWire>>,
    pub commit_diff_request_id: ReadSignal<Option<String>>,
    pub set_commit_diff_request_id: WriteSignal<Option<String>>,
    pub commit_diff_result: ReadSignal<Vec<CommitFileDiff>>,
    pub set_commit_diff_result: WriteSignal<Vec<CommitFileDiff>>,

    // 文件树 (增量更新)
    pub tree_nodes: ReadSignal<Vec<FileNode>>,
    pub set_tree_nodes: WriteSignal<Vec<FileNode>>,

    // Dashboard 系统指标
    pub system_metrics: ReadSignal<Option<SystemMetricsData>>,
    pub set_system_metrics: WriteSignal<Option<SystemMetricsData>>,

    // 浏览器存储降级
    pub degraded_sync_mode: ReadSignal<Option<DegradedSyncMode>>,
    pub set_degraded_sync_mode: WriteSignal<Option<DegradedSyncMode>>,
    pub sync_banner: ReadSignal<Option<String>>,
    pub set_sync_banner: WriteSignal<Option<String>>,

    pub set_explicit_home: WriteSignal<bool>,
}
