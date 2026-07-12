// apps/web/src/editor/hook.rs
//! plan_ref:
//!   - 10_rendering#large-document-runtime
//!   - 10_rendering#document-authority-bridge
//!
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
use super::hook_effects::setup_editor_effects;
use super::hook_runtime::build_editor_runtime;
use crate::api::WsService;
use crate::hooks::use_core::EditorContext;
use crate::runtime::domain::EditorSyncFailure;
use deve_core::models::DocId;
use leptos::html::Div;
use leptos::prelude::*;

pub(super) struct EditorState {
    pub content: ReadSignal<String>,
    pub playback_version: ReadSignal<u64>,
    pub open_request_id: ReadSignal<u64>,
    pub sync_failure: ReadSignal<Option<EditorSyncFailure>>,
    pub retry_sync: Callback<()>,
}

pub(super) fn use_editor(
    doc_id: DocId,
    editor_ref: NodeRef<Div>,
    on_stats: Option<Callback<EditorStats>>,
) -> EditorState {
    let ws = use_context::<WsService>().expect("WsService should be provided");
    let core = expect_context::<EditorContext>();
    let runtime = build_editor_runtime(&core);

    setup_editor_effects(
        &runtime,
        ws.clone(),
        core.clone(),
        doc_id,
        editor_ref,
        on_stats.clone(),
    );

    let set_failure = runtime.set_editor_sync_failure;
    let set_reopen_attempted = runtime.set_snapshot_reopen_attempted;
    let set_last_open_request_key = runtime.set_last_open_request_key;
    let set_retry_nonce = runtime.set_retry_nonce;
    let retry_sync = Callback::new(move |_| {
        super::ffi::set_read_only(true);
        set_failure.set(None);
        set_reopen_attempted.set(false);
        set_last_open_request_key.set(None);
        set_retry_nonce.update(|nonce| *nonce = nonce.wrapping_add(1));
    });

    EditorState {
        content: runtime.content,
        playback_version: runtime.playback_version,
        open_request_id: runtime.open_request_id,
        sync_failure: runtime.editor_sync_failure,
        retry_sync,
    }
}
