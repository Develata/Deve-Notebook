use leptos::prelude::*;
use crate::api::WsService;
use std::collections::HashMap;
use deve_core::models::{DocId, PeerId, VersionVector};
use deve_core::source_control::{ChangeEntry, CommitInfo};
use crate::editor::EditorStats;

#[derive(Clone, Debug, PartialEq)]
pub struct PeerSession {
    pub id: PeerId,
    pub vector: VersionVector,
    pub last_seen: u64, // timestamp
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
    pub plugin_last_response: ReadSignal<Option<(String, Option<serde_json::Value>, Option<String>)>>,
    pub on_plugin_call: Callback<(String, String, String, Vec<serde_json::Value>)>,

    // 搜索
    pub search_results: ReadSignal<Vec<(String, String, f32)>>, // (doc_id, path, score)
    pub on_search: Callback<String>,
    
    // 手动合并
    pub sync_mode: ReadSignal<String>, // "auto" or "manual"
    pub pending_ops_count: ReadSignal<u32>,
    pub pending_ops_previews: ReadSignal<Vec<(String, String, String)>>,
    pub on_get_sync_mode: Callback<()>,
    pub on_set_sync_mode: Callback<String>,
    pub on_get_pending_ops: Callback<()>,
    pub on_confirm_merge: Callback<()>,
    pub on_discard_pending: Callback<()>,
    
    // 版本控制状态 (Repo)
    pub active_repo: ReadSignal<Option<PeerId>>,
    pub set_active_repo: WriteSignal<Option<PeerId>>,
    
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
    pub on_commit: Callback<String>,
    pub on_get_history: Callback<u32>,
    pub diff_content: ReadSignal<Option<(String, String, String)>>,
    pub set_diff_content: WriteSignal<Option<(String, String, String)>>,
    pub on_get_doc_diff: Callback<String>,
}
