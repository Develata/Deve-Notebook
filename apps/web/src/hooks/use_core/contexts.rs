// apps/web/src/hooks/use_core/contexts.rs
//! 子上下文定义：将 CoreState 按领域拆分为 6 个独立上下文。
//! 组件按需 `expect_context::<XxxContext>()` 而非依赖单一巨型结构。
//!
//! 部分字段暂未被组件直接读取（仅通过 CoreState 兼容层使用），
//! 待后续组件全部迁移后自然消除。
#![allow(dead_code)]

use crate::editor::EditorStats;
use super::diff_session::DiffSessionWire;
use super::state::PluginResponse;
use super::types::ChatMessage;
use deve_core::models::{DocId, PeerId};
use deve_core::source_control::{ChangeEntry, CommitInfo};
use deve_core::tree::FileNode;
use leptos::prelude::*;

/// 文档与文件树上下文
#[derive(Clone)]
pub struct DocContext {
    pub docs: ReadSignal<Vec<(DocId, String)>>,
    pub current_doc: ReadSignal<Option<DocId>>,
    pub set_current_doc: WriteSignal<Option<DocId>>,
    pub tree_nodes: ReadSignal<Vec<FileNode>>,
    pub on_doc_select: Callback<DocId>,
    pub on_doc_create: Callback<String>,
    pub on_doc_rename: Callback<(String, String)>,
    pub on_doc_delete: Callback<String>,
    pub on_doc_copy: Callback<(String, String)>,
    pub on_doc_move: Callback<(String, String)>,
    pub search_results: ReadSignal<Vec<(String, String, f32)>>,
    pub on_search: Callback<String>,
}

/// 编辑器状态上下文
#[derive(Clone)]
pub struct EditorContext {
    pub stats: ReadSignal<EditorStats>,
    pub on_stats: Callback<EditorStats>,
    pub load_state: ReadSignal<String>,
    pub set_load_state: WriteSignal<String>,
    pub load_progress: ReadSignal<(usize, usize)>,
    pub set_load_progress: WriteSignal<(usize, usize)>,
    pub load_eta_ms: ReadSignal<u64>,
    pub set_load_eta_ms: WriteSignal<u64>,
    pub doc_version: ReadSignal<u64>,
    pub set_doc_version: WriteSignal<u64>,
    pub playback_version: ReadSignal<u64>,
    pub set_playback_version: WriteSignal<u64>,
    pub is_spectator: Signal<bool>,
}

/// AI 聊天与插件上下文
#[derive(Clone)]
pub struct ChatContext {
    pub messages: ReadSignal<Vec<ChatMessage>>,
    pub set_messages: WriteSignal<Vec<ChatMessage>>,
    pub is_streaming: ReadSignal<bool>,
    pub set_is_streaming: WriteSignal<bool>,
    pub ai_mode: ReadSignal<String>,
    pub set_ai_mode: WriteSignal<String>,
    pub plugin_last_response: ReadSignal<PluginResponse>,
    pub on_plugin_call: Callback<(String, String, String, Vec<serde_json::Value>)>,
}

/// 同步 / 合并上下文
#[derive(Clone)]
pub struct SyncMergeContext {
    pub sync_mode: ReadSignal<String>,
    pub pending_ops_count: ReadSignal<u32>,
    pub pending_ops_previews: ReadSignal<Vec<(String, String, String)>>,
    pub on_get_sync_mode: Callback<()>,
    pub on_set_sync_mode: Callback<String>,
    pub on_get_pending_ops: Callback<()>,
    pub on_confirm_merge: Callback<()>,
    pub on_discard_pending: Callback<()>,
    pub on_merge_peer: Callback<String>,
}

/// 版本控制 (Source Control) 上下文
#[derive(Clone)]
pub struct SourceControlContext {
    pub staged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub unstaged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub commit_history: ReadSignal<Vec<CommitInfo>>,
    pub on_get_changes: Callback<()>,
    pub on_stage_file: Callback<String>,
    pub on_stage_files: Callback<Vec<String>>,
    pub on_unstage_file: Callback<String>,
    pub on_unstage_files: Callback<Vec<String>>,
    pub on_discard_file: Callback<String>,
    pub on_commit: Callback<String>,
    pub on_get_history: Callback<u32>,
    pub diff_content: ReadSignal<Option<DiffSessionWire>>,
    pub set_diff_content: WriteSignal<Option<DiffSessionWire>>,
    pub on_get_doc_diff: Callback<String>,
}

/// 分支 / 仓库上下文
#[derive(Clone)]
pub struct BranchContext {
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub set_active_branch: WriteSignal<Option<PeerId>>,
    pub on_switch_branch: Callback<Option<String>>,
    pub current_repo: ReadSignal<Option<String>>,
    pub set_current_repo: WriteSignal<Option<String>>,
    pub on_switch_repo: Callback<String>,
    pub shadow_repos: ReadSignal<Vec<String>>,
    pub on_list_shadows: Callback<()>,
    pub repo_list: ReadSignal<Vec<String>>,
}
