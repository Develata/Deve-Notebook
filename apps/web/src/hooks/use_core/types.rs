// apps\web\src\hooks\use_core

#![allow(dead_code)] // CoreState 字段为未来功能预留

use crate::api::WsService;
use crate::editor::EditorStats;
use deve_core::models::{DocId, PeerId, VersionVector};
use deve_core::source_control::{ChangeEntry, CommitInfo};
use deve_core::tree::FileNode;
use leptos::prelude::*;
use std::collections::HashMap;

use super::state::PluginResponse;

#[derive(Clone, Debug, PartialEq)]
pub struct PeerSession {
    pub id: PeerId,
    pub vector: VersionVector,
    pub last_seen: u64, // timestamp
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChatMessage {
    pub role: String, // "user" or "assistant"
    pub content: String,
    pub req_id: Option<String>, // To link with streaming chunks
    pub ts_ms: u64,
}

#[derive(Clone)]
pub struct CoreState {
    pub ws: WsService,
    pub docs: ReadSignal<Vec<(DocId, String)>>,
    pub current_doc: ReadSignal<Option<DocId>>,
    pub set_current_doc: WriteSignal<Option<DocId>>,
    pub status_text: Signal<String>,
    pub stats: ReadSignal<EditorStats>,

    // P2P 状态
    pub peers: ReadSignal<HashMap<PeerId, PeerSession>>,

    pub on_doc_select: Callback<DocId>,
    pub on_doc_create: Callback<String>,
    pub on_doc_rename: Callback<(String, String)>,
    pub on_doc_delete: Callback<String>,
    pub on_doc_copy: Callback<(String, String)>,
    pub on_doc_move: Callback<(String, String)>,
    pub on_stats: Callback<EditorStats>,

    // 插件 RPC
    pub plugin_last_response: ReadSignal<PluginResponse>,
    pub on_plugin_call: Callback<(String, String, String, Vec<serde_json::Value>)>,

    // AI Chat State
    pub chat_messages: ReadSignal<Vec<ChatMessage>>,
    pub set_chat_messages: WriteSignal<Vec<ChatMessage>>, // Used by effect logic (in mod.rs, we'll wrap helpers)
    pub is_chat_streaming: ReadSignal<bool>,
    pub set_is_chat_streaming: WriteSignal<bool>,
    pub ai_mode: ReadSignal<String>,
    pub set_ai_mode: WriteSignal<String>,

    // 搜索
    pub search_results: ReadSignal<Vec<(String, String, f32)>>, // (doc_id, path, score)
    pub on_search: Callback<String>,

    // 文档加载状态
    pub load_state: ReadSignal<String>,
    pub set_load_state: WriteSignal<String>,
    pub load_progress: ReadSignal<(usize, usize)>,
    pub set_load_progress: WriteSignal<(usize, usize)>,
    pub load_eta_ms: ReadSignal<u64>,
    pub set_load_eta_ms: WriteSignal<u64>,

    // 手动合并
    pub sync_mode: ReadSignal<String>, // "auto" or "manual"
    pub pending_ops_count: ReadSignal<u32>,
    pub pending_ops_previews: ReadSignal<Vec<(String, String, String)>>,
    pub on_get_sync_mode: Callback<()>,
    pub on_set_sync_mode: Callback<String>,
    pub on_get_pending_ops: Callback<()>,
    pub on_confirm_merge: Callback<()>,
    pub on_discard_pending: Callback<()>,

    // 分支状态 (Branch -> Peer)
    pub active_branch: ReadSignal<Option<PeerId>>,
    pub set_active_branch: WriteSignal<Option<PeerId>>,
    pub on_switch_branch: Callback<Option<String>>, // None for Local

    // 仓库状态 (Repo -> .redb File)
    pub current_repo: ReadSignal<Option<String>>,
    pub set_current_repo: WriteSignal<Option<String>>,
    pub on_switch_repo: Callback<String>,

    // 分支切换状态
    pub shadow_repos: ReadSignal<Vec<String>>,
    pub on_list_shadows: Callback<()>,

    // 仓库列表 (当前分支下的 .redb 文件)
    pub repo_list: ReadSignal<Vec<String>>,

    // 版本控制状态 (历史)
    pub doc_version: ReadSignal<u64>, // 当前最大版本
    pub set_doc_version: WriteSignal<u64>,
    pub playback_version: ReadSignal<u64>, // 当前回放视图版本
    pub set_playback_version: WriteSignal<u64>,

    // 旁观者模式 (Spectator Mode)
    pub is_spectator: Signal<bool>,

    // Source Control (New)
    pub staged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub unstaged_changes: ReadSignal<Vec<ChangeEntry>>,
    pub commit_history: ReadSignal<Vec<CommitInfo>>,
    pub on_get_changes: Callback<()>,
    pub on_stage_file: Callback<String>,
    pub on_unstage_file: Callback<String>,
    pub on_discard_file: Callback<String>,
    pub on_commit: Callback<String>,
    pub on_get_history: Callback<u32>,
    pub diff_content: ReadSignal<Option<(String, String, String)>>,
    pub set_diff_content: WriteSignal<Option<(String, String, String)>>,
    pub on_get_doc_diff: Callback<String>,
    pub on_merge_peer: Callback<String>,

    // 文件树 (增量更新)
    pub tree_nodes: ReadSignal<Vec<FileNode>>,
}

impl CoreState {
    pub fn append_chat_message(&self, role: &str, content: &str, req_id: Option<String>) {
        self.set_chat_messages.update(|msgs| {
            msgs.push(ChatMessage {
                role: role.to_string(),
                content: content.to_string(),
                req_id,
                ts_ms: js_sys::Date::now() as u64,
            });
        });
    }

    pub fn update_chat_message(&self, req_id: &str, delta: &str) {
        self.set_chat_messages.update(|msgs| {
            if let Some(msg) = msgs
                .iter_mut()
                .rev()
                .find(|m| m.req_id.as_deref() == Some(req_id))
            {
                msg.content.push_str(delta);
            } else {
                // If not found (race), append new
                msgs.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: delta.to_string(),
                    req_id: Some(req_id.to_string()),
                    ts_ms: js_sys::Date::now() as u64,
                });
            }
        });
    }
}
