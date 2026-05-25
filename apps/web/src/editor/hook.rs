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
use deve_core::models::DocId;
use leptos::html::Div;
use leptos::prelude::*;

pub(super) struct EditorState {
    pub content: ReadSignal<String>,
    pub playback_version: ReadSignal<u64>,
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

    EditorState {
        content: runtime.content,
        playback_version: runtime.playback_version,
    }
}
