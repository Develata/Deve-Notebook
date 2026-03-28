// apps/web/src/editor/hook.rs
//! # Editor Hook (编辑器钩子)
//!
//! **架构作用**:
//! 封装编辑器的状态管理逻辑 (`use_editor`)。
//! 包含文档加载、WebSocket 消息处理协调、CodeMirror 初始化和更新循环。
//!
//! ## 性能优化 (v4)
//! - 使用 Delta 模式: JS 只发送变更，不再发送全文
//! - 避免了 JS->WASM 全文拷贝和 Rust 端 Diff 计算
//! - 添加了 `on_cleanup` 确保编辑器资源正确释放

use super::EditorStats;
use super::handshake_reset::{HandshakeResetCtx, setup_handshake_reset_effect};
use super::hook_editor::{
    EditorCleanupCtx, EditorMountEffectCtx, setup_editor_cleanup, setup_editor_mount_effect,
};
use super::hook_open::{OpenDocEffectCtx, setup_open_doc_effect};
use super::hook_playback::{PlaybackEffectCtx, setup_playback_effect};
use super::message_effect;
use super::open_scope::OpenRequestKey;
use super::request_key::setup_request_key_effect;
use crate::api::WsService;
use crate::hooks::use_core::EditorContext;
use deve_core::models::DocId;
use deve_core::security::{EncryptedOp, RepoKey};
use leptos::html::Div;
use leptos::prelude::*;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

#[allow(dead_code)] // 为回放功能预留的字段
pub struct EditorState {
    pub content: ReadSignal<String>,
    pub is_playback: ReadSignal<bool>,
    pub playback_version: ReadSignal<u64>,
    pub local_version: ReadSignal<u64>,
    pub on_playback_change: Box<dyn Fn(u64) + Send + Sync>,
}

pub fn use_editor(
    doc_id: DocId,
    editor_ref: NodeRef<Div>,
    on_stats: Option<Callback<EditorStats>>,
) -> EditorState {
    let ws = use_context::<WsService>().expect("WsService should be provided");
    let core = expect_context::<EditorContext>();

    // 文档的本地状态
    let (content, set_content) = signal("".to_string());
    let (local_version, set_local_version) = signal(0u64);
    let (open_request_id, set_open_request_id) = signal(0u64);
    let (last_open_request_key, set_last_open_request_key) = signal(None::<OpenRequestKey>);
    let session_generation = Arc::new(AtomicU64::new(0));
    let ready_generation = Arc::new(AtomicU64::new(0));
    let buffered_live_ops = Arc::new(Mutex::new(Vec::new()));
    let buffered_encrypted_ops = Arc::new(Mutex::new(Vec::<EncryptedOp>::new()));
    let set_doc_ver = core.set_doc_version;

    // 回放状态
    let (history, set_history) = signal(Vec::<(u64, deve_core::models::Op)>::new());
    let playback_version = core.playback_version;
    let set_playback_version = core.set_playback_version;

    let (is_playback, set_is_playback) = signal(false);

    // E2EE: RepoKey 信号 (RAM-only, 页面卸载时自动清除)
    let (repo_key, set_repo_key) = signal(None::<RepoKey>);

    setup_open_doc_effect(OpenDocEffectCtx {
        ws: ws.clone(),
        core: core.clone(),
        doc_id,
        last_open_request_key,
        set_last_open_request_key,
        session_generation: session_generation.clone(),
        ready_generation: ready_generation.clone(),
        buffered_live_ops: buffered_live_ops.clone(),
        buffered_encrypted_ops: buffered_encrypted_ops.clone(),
        set_local_version,
        set_open_request_id,
        set_history,
    });

    setup_request_key_effect(ws.clone(), core.clone(), set_repo_key);

    // 同步本地版本到 Core
    Effect::new(move |_| {
        let ver = local_version.get();
        set_doc_ver.set(ver);
    });

    message_effect::setup_server_message_effect(message_effect::ServerMessageEffectCtx {
        ws: ws.clone(),
        core: core.clone(),
        doc_id,
        open_request_id,
        session_generation: session_generation.clone(),
        ready_generation: ready_generation.clone(),
        buffered_live_ops: buffered_live_ops.clone(),
        buffered_encrypted_ops: buffered_encrypted_ops.clone(),
        set_content,
        local_version,
        set_local_version,
        history,
        set_history,
        is_playback,
        set_playback_version,
        on_stats: on_stats.clone(),
        repo_key,
        set_repo_key,
    });
    setup_handshake_reset_effect(HandshakeResetCtx {
        ws: ws.clone(),
        core: core.clone(),
        ready_generation: ready_generation.clone(),
        buffered_live_ops: buffered_live_ops.clone(),
        buffered_encrypted_ops: buffered_encrypted_ops.clone(),
        set_repo_key,
    });

    setup_editor_mount_effect(EditorMountEffectCtx {
        doc_id,
        editor_ref,
        ws: ws.clone(),
        core: core.clone(),
        is_playback,
        local_version,
        on_stats,
        set_content,
    });
    setup_editor_cleanup(EditorCleanupCtx {
        session_generation,
        ready_generation,
        buffered_live_ops,
        buffered_encrypted_ops,
    });
    setup_playback_effect(PlaybackEffectCtx {
        ws: ws.clone(),
        core: core.clone(),
        doc_id,
        history,
        playback_version,
        local_version,
        set_is_playback,
    });

    let on_playback_change = Box::new(move |ver: u64| {
        set_playback_version.set(ver);
    });

    EditorState {
        content,
        is_playback,
        playback_version,
        local_version,
        on_playback_change,
    }
}
