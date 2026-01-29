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
//! - `callbacks_sc`: Source Control 回调 (已拆分)

pub mod apply;
pub mod callbacks;
pub mod callbacks_sc;
pub mod effects;
pub mod effects_msg;
pub mod state;
pub mod types;

pub use types::*;

use crate::api::WsService;
use base64::Engine;
use leptos::prelude::*;
use std::sync::Arc;

const IDENTITY_KEY_STORAGE: &str = "deve_note_identity_key";

/// 从 localStorage 加载或生成新的身份密钥对
///
/// **持久化策略**:
/// - 首次访问时生成新密钥并存入 localStorage
/// - 后续访问从 localStorage 恢复，保持 PeerId 一致
/// - 避免后端 VersionVector 因一次性 PeerId 无限膨胀
fn load_or_generate_identity() -> deve_core::security::IdentityKeyPair {
    let window = web_sys::window().expect("no global window");
    let storage = window
        .local_storage()
        .ok()
        .flatten()
        .expect("localStorage not available");

    // 尝试从 localStorage 加载
    if let Ok(Some(encoded)) = storage.get_item(IDENTITY_KEY_STORAGE) {
        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&encoded) {
            if let Some(key_pair) = deve_core::security::IdentityKeyPair::from_bytes(&bytes) {
                leptos::logging::log!("Identity loaded from localStorage");
                return key_pair;
            }
        }
        leptos::logging::warn!("Failed to decode stored identity, regenerating...");
    }

    // 生成新密钥并存储
    let key_pair = deve_core::security::IdentityKeyPair::generate();
    let encoded = base64::engine::general_purpose::STANDARD.encode(key_pair.to_bytes());
    if let Err(e) = storage.set_item(IDENTITY_KEY_STORAGE, &encoded) {
        leptos::logging::error!("Failed to save identity to localStorage: {:?}", e);
    } else {
        leptos::logging::log!("New identity generated and saved to localStorage");
    }

    key_pair
}

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

    // 3. 加载或生成持久化的 Identity KeyPair (修复向量膨胀问题)
    let key_pair = Arc::new(load_or_generate_identity());
    let peer_id = key_pair.peer_id();
    leptos::logging::log!("Frontend PeerId: {}", peer_id);

    // 4. 设置 Effects
    effects::setup_handshake_effect(&ws, key_pair.clone(), peer_id.clone());
    effects::setup_message_effect(&ws, &signals);

    // 5. 创建回调
    let doc_callbacks = callbacks::create_doc_callbacks(&ws, signals.set_current_doc);
    let sync_callbacks = callbacks::create_sync_callbacks(&ws, signals.current_doc);
    let sc_callbacks = callbacks::create_source_control_callbacks(&ws);
    let misc_callbacks = callbacks::create_misc_callbacks(&ws, signals.set_stats);
    let switch_callbacks = callbacks::create_switch_callbacks(&ws);

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
        active_branch: signals.active_branch,
        set_active_branch: signals.set_active_branch,
        on_switch_branch: switch_callbacks.on_switch_branch,
        current_repo: signals.current_repo,
        set_current_repo: signals.set_current_repo,
        on_switch_repo: switch_callbacks.on_switch_repo,
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
        on_discard_file: sc_callbacks.on_discard_file,
        on_commit: sc_callbacks.on_commit,
        on_get_history: sc_callbacks.on_get_history,
        diff_content: signals.diff_content,
        set_diff_content: signals.set_diff_content,
        on_get_doc_diff: sc_callbacks.on_get_doc_diff,
        on_merge_peer: sync_callbacks.on_merge_peer,
        tree_nodes: signals.tree_nodes,
        chat_messages: signals.chat_messages,
        set_chat_messages: signals.set_chat_messages,
        is_chat_streaming: signals.is_chat_streaming,
        set_is_chat_streaming: signals.set_is_chat_streaming,
        ai_mode: signals.ai_mode,
        set_ai_mode: signals.set_ai_mode,
    };

    // 7. 提供上下文
    provide_context(state.clone());

    state
}
