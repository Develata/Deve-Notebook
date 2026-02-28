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
use super::ffi::{Delta, destroyEditor, set_read_only, setupCodeMirror};
use super::playback;
use super::sync;
use crate::api::{ConnectionStatus, WsService};
use crate::hooks::use_core::EditorContext;
use deve_core::models::DocId;
use deve_core::protocol::ClientMessage;
use deve_core::security::RepoKey;
use leptos::html::Div;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

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
    let set_load_state = core.set_load_state;
    let set_load_progress = core.set_load_progress;
    let set_load_eta_ms = core.set_load_eta_ms;

    // 回放状态
    let (history, set_history) = signal(Vec::<(u64, deve_core::models::Op)>::new());
    let playback_version = core.playback_version;
    let set_playback_version = core.set_playback_version;

    let (is_playback, set_is_playback) = signal(false);

    // E2EE: RepoKey 信号 (RAM-only, 页面卸载时自动清除)
    let (repo_key, set_repo_key) = signal(None::<RepoKey>);

    // 生成会话 client_id
    let client_id = (js_sys::Math::random() * 1_000_000.0) as u64;

    // 初始请求: 打开文档
    let ws_clone = ws.clone();
    let set_doc_ver = core.set_doc_version;
    Effect::new(move |_| {
        set_content.set("Loading...".to_string());
        set_local_version.set(0);
        set_history.set(Vec::new());
        set_doc_ver.set(0);
        set_playback_version.set(0);
        set_load_state.set("loading".to_string());
        set_load_progress.set((0, 0));
        set_load_eta_ms.set(0);
        ws_clone.send(ClientMessage::OpenDoc { doc_id });
    });

    // E2EE: 连接成功后请求 RepoKey
    let ws_key = ws.clone();
    Effect::new(move |_| {
        if ws_key.status.get() == ConnectionStatus::Connected {
            ws_key.send(ClientMessage::RequestKey);
        }
    });

    // 同步本地版本到 Core
    Effect::new(move |_| {
        let ver = local_version.get();
        set_doc_ver.set(ver);
    });

    // 处理传入消息
    let ws_clone_2 = ws.clone();
    Effect::new(move |_| {
        if let Some(msg) = ws_clone_2.msg.get() {
            let ctx = sync::context::SyncContext {
                doc_id,
                client_id,
                ws: &ws_clone_2,
                set_content,
                local_version,
                set_local_version,
                set_history,
                is_playback,
                set_playback_version,
                set_load_state,
                set_load_progress,
                set_load_eta_ms,
                on_stats,
                repo_key,
                set_repo_key,
            };
            sync::handle_server_message(msg, &ctx);
        }
    });

    // 编辑器初始化 (Delta 模式)
    Effect::new(move |_| {
        if let Some(element) = editor_ref.get() {
            let raw_element: &web_sys::HtmlElement = &element;
            let ws_for_update = ws.clone();

            // Delta 回调: 接收 JSON 格式的变更数组
            let on_delta = Closure::wrap(Box::new(move |delta_json: String| {
                // 回放模式时忽略
                if is_playback.get_untracked() {
                    return;
                }

                // 解析 Delta 数组
                let deltas: Vec<Delta> = match serde_json::from_str(&delta_json) {
                    Ok(d) => d,
                    Err(e) => {
                        leptos::logging::error!("Delta 解析失败: {:?}", e);
                        return;
                    }
                };

                // 转换 Delta 为 Op 并发送
                for delta in deltas {
                    let ops = delta.to_ops();
                    for op in ops {
                        ws_for_update.send(ClientMessage::Edit {
                            doc_id,
                            op: op.clone(),
                            client_id,
                        });
                    }
                }

                // 更新本地内容信号
                let txt = super::ffi::getEditorContent();

                // 计算统计信息
                if let Some(cb) = on_stats {
                    let lines = txt.lines().count();
                    let words = txt.split_whitespace().count();
                    cb.run(EditorStats {
                        chars: txt.len(),
                        words,
                        lines,
                    });
                }

                set_content.set(txt);
            }) as Box<dyn FnMut(String)>);

            setupCodeMirror(raw_element, &on_delta);
            // Store the closure so it gets dropped on cleanup instead of leaking
            let on_delta = StoredValue::new_local(Some(on_delta));
            on_cleanup(move || {
                // Drop the closure to prevent memory leak
                on_delta.update_value(|v| {
                    v.take();
                });
            });
        }
    });

    // 清理: 组件卸载时销毁编辑器
    on_cleanup(move || {
        destroyEditor();
        leptos::logging::log!("编辑器已清理");
    });

    // 回放逻辑
    Effect::new(move |_| {
        let ver = playback_version.get();
        let local = local_version.get_untracked();

        playback::handle_playback_change(ver, doc_id, local, history, set_is_playback);

        let is_pb = ver < local;
        let spectator = core.is_spectator.get_untracked();
        let loading = core.load_state.get_untracked() != "ready";
        let should_readonly = is_pb || spectator || loading;
        set_read_only(should_readonly);
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
