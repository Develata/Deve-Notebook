// apps/web/src/hooks/use_core/mod.rs
//! # Core State Hook (核心状态钩子)
//!
//! **架构作用**:
//! 管理前端全局核心状态，连接 WebSocket 服务与 UI 组件。
//!
//! ## 子模块
//! - `types`: 类型定义
//! - `state`: 信号声明
//! - `effects`: 响应式效果
//! - `callbacks`: 用户交互回调

pub mod callbacks;
pub mod effects;
pub mod state;
pub mod types;

pub use types::*;

use crate::api::WsService;
use leptos::prelude::*;
use std::sync::Arc;

/// 初始化核心状态钩子
///
/// 返回 `CoreState`，包含所有信号和回调。
pub fn use_core() -> CoreState {
    // 1. 初始化 WebSocket 服务
    let ws = WsService::new();
    provide_context(ws.clone());

    let status_signal_for_text = ws.status;
    let status_text = Signal::derive(move || format!("{}", status_signal_for_text.get()));

    // 2. 初始化所有信号
    let signals = state::init_signals();

    // 3. 生成临时 Identity KeyPair
    let key_pair = Arc::new(deve_core::security::IdentityKeyPair::generate());
    let peer_id = key_pair.peer_id();
    leptos::logging::log!("Frontend PeerId: {}", peer_id);

    // 4. 设置 Effects
    effects::setup_handshake_effect(&ws, key_pair.clone(), peer_id.clone());
    effects::setup_message_effect(&ws, &signals);
    effects::setup_branch_switch_effect(&ws, signals.active_repo);

    // 5. 创建回调
    let doc_callbacks = callbacks::create_doc_callbacks(&ws, signals.set_current_doc);
    let sync_callbacks = callbacks::create_sync_callbacks(&ws, signals.current_doc);
    let sc_callbacks = callbacks::create_source_control_callbacks(&ws);
    let misc_callbacks = callbacks::create_misc_callbacks(&ws, signals.set_stats);

    // 6. 组装最终状态
    let state = CoreState {
        ws,
        docs: signals.docs,
        current_doc: signals.current_doc,
        set_current_doc: signals.set_current_doc,
        status_text,
        stats: signals.stats,
        peers: signals.peers,
        on_doc_select: doc_callbacks.on_doc_select,
        on_doc_create: doc_callbacks.on_doc_create,
        on_doc_rename: doc_callbacks.on_doc_rename,
        on_doc_delete: doc_callbacks.on_doc_delete,
        on_doc_copy: doc_callbacks.on_doc_copy,
        on_doc_move: doc_callbacks.on_doc_move,
        on_stats: misc_callbacks.on_stats,
        plugin_last_response: signals.plugin_response,
        on_plugin_call: misc_callbacks.on_plugin_call,
        search_results: signals.search_results,
        on_search: misc_callbacks.on_search,
        sync_mode: signals.sync_mode,
        pending_ops_count: signals.pending_ops_count,
        pending_ops_previews: signals.pending_ops_previews,
        on_get_sync_mode: sync_callbacks.on_get_sync_mode,
        on_set_sync_mode: sync_callbacks.on_set_sync_mode,
        on_get_pending_ops: sync_callbacks.on_get_pending_ops,
        on_confirm_merge: sync_callbacks.on_confirm_merge,
        on_discard_pending: sync_callbacks.on_discard_pending,
        active_repo: signals.active_repo,
        set_active_repo: signals.set_active_repo,
        shadow_repos: signals.shadow_repos,
        on_list_shadows: sync_callbacks.on_list_shadows,
        repo_list: signals.repo_list,
        doc_version: signals.doc_version,
        set_doc_version: signals.set_doc_version,
        playback_version: signals.playback_version,
        set_playback_version: signals.set_playback_version,
        is_spectator: signals.is_spectator.into(),
        staged_changes: signals.staged_changes,
        unstaged_changes: signals.unstaged_changes,
        commit_history: signals.commit_history,
        on_get_changes: sc_callbacks.on_get_changes,
        on_stage_file: sc_callbacks.on_stage_file,
        on_unstage_file: sc_callbacks.on_unstage_file,
        on_commit: sc_callbacks.on_commit,
        on_get_history: sc_callbacks.on_get_history,
        diff_content: signals.diff_content,
        set_diff_content: signals.set_diff_content,
        on_get_doc_diff: sc_callbacks.on_get_doc_diff,
        on_merge_peer: sync_callbacks.on_merge_peer,
    };

    // 7. 提供上下文
    provide_context(state.clone());

    state
}
